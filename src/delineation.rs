//! Delineation: Text to Protoform (the structural pass).
//!
//! The delineator reads text into protoforms using an explicit stack,
//! bounded by DEPTH_LIMIT, so that any text yields a Delineation or
//! a Fault and never aborts the process.

use crate::{
    Boundary, Delimiting, Delineation, Enclosure, Extent, Fault, Glyphing, Head, Identifying,
    Integer, Path, Problem, Protoform, Protosizable, Recognizing, Separator, Situation, Text,
};

const DEPTH_LIMIT: usize = 100_000;

// ---------------------------------------------------------------------------
// Bare run parsing: iterative, no recursion
// ---------------------------------------------------------------------------

struct SplitPoint {
    byte_offset: usize,
    separator: Separator,
}

/// The kind whose capability parses bare runs into protoforms.
trait BareRunParsing {
    fn find_split_points(run: &str) -> Vec<SplitPoint>;
    fn parse_bare_run(
        run: &str,
        run_start: usize,
        base_path: &[Integer],
    ) -> (Protoform, Vec<(Path, Extent)>);
}

struct BareRunParser;

impl BareRunParsing for BareRunParser {
    fn find_split_points(run: &str) -> Vec<SplitPoint> {
        let chars: Vec<(usize, char)> = run.char_indices().collect();
        let mut points = Vec::new();
        for i in 0..chars.len() {
            let (byte_offset, c) = chars[i];
            if let Some(sep) = Separator::identify(c) {
                // A valid split point: not at edges, neighbors are not separators
                if i == 0 || i == chars.len() - 1 {
                    continue;
                }
                if Separator::identify(chars[i - 1].1).is_some() {
                    continue;
                }
                if Separator::identify(chars[i + 1].1).is_some() {
                    continue;
                }
                points.push(SplitPoint {
                    byte_offset,
                    separator: sep,
                });
            }
        }
        points
    }

    fn parse_bare_run(
        run: &str,
        run_start: usize,
        base_path: &[Integer],
    ) -> (Protoform, Vec<(Path, Extent)>) {
        let points = Self::find_split_points(run);
        let run_end = run_start + run.len();
        let full_extent = Extent(run_start as Integer, run_end as Integer);
        let mut situations = vec![(base_path.to_vec(), full_extent)];

        if points.is_empty() {
            return (Protoform::Bare(Head::Bare(run.to_owned())), situations);
        }

        // Collect segments: (head_text, separator, body_start_byte_in_source)
        let mut segments: Vec<(&str, Separator, usize)> = Vec::new();
        let mut remaining = run;
        let mut offset = 0usize;
        for point in &points {
            let local_offset = point.byte_offset - offset;
            let head_text = &remaining[..local_offset];
            let sep_len = point.separator.glyph().len_utf8();
            let body_start = run_start + point.byte_offset + sep_len;
            segments.push((head_text, point.separator, body_start));
            remaining = &remaining[local_offset + sep_len..];
            offset = point.byte_offset + sep_len;
        }

        // Compute sub-situations: each body at [base, 0, 0, ...]
        for (depth, segment) in segments.iter().enumerate() {
            let mut body_path = base_path.to_vec();
            body_path.extend(std::iter::repeat_n(0, depth + 1));
            situations.push((body_path, Extent(segment.2 as Integer, run_end as Integer)));
        }

        // Build chain from right to left
        let mut result = Protoform::Bare(Head::Bare(remaining.to_owned()));
        for (head_text, sep, _) in segments.into_iter().rev() {
            result = Protoform::Headed(Head::Bare(head_text.to_owned()), sep, Box::new(result));
        }

        (result, situations)
    }
}

// ---------------------------------------------------------------------------
// attach_body: iterative, replaces the deepest Bare with a Headed
// ---------------------------------------------------------------------------

/// The kind whose capability attaches a body to the deepest node of a chain.
trait Attaching {
    fn attach_body(chain: Protoform, separator: Separator, body: Protoform) -> Protoform;
    fn chain_depth(pf: &Protoform) -> usize;
}

struct BodyAttacher;

impl Attaching for BodyAttacher {
    fn attach_body(chain: Protoform, separator: Separator, body: Protoform) -> Protoform {
        let mut segments: Vec<(Head, Separator)> = Vec::new();
        let mut current = chain;
        loop {
            match current {
                Protoform::Headed(h, s, inner) => {
                    segments.push((h, s));
                    current = *inner;
                }
                Protoform::Bare(h) => {
                    let mut result = Protoform::Headed(h, separator, Box::new(body));
                    for (h, s) in segments.into_iter().rev() {
                        result = Protoform::Headed(h, s, Box::new(result));
                    }
                    return result;
                }
                other => {
                    let _ = other;
                    let mut result =
                        Protoform::Headed(Head::Bare(String::new()), separator, Box::new(body));
                    for (h, s) in segments.into_iter().rev() {
                        result = Protoform::Headed(h, s, Box::new(result));
                    }
                    return result;
                }
            }
        }
    }

    fn chain_depth(pf: &Protoform) -> usize {
        let mut depth = 0;
        let mut current = pf;
        loop {
            match current {
                Protoform::Headed(_, _, body) => {
                    depth += 1;
                    current = body.as_ref();
                }
                _ => return depth,
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Frame: one nesting level on the explicit stack
// ---------------------------------------------------------------------------

enum FrameKind {
    Root,
    Enclosure(Enclosure),
    QualifiedAngle(String),
}

struct Pending {
    chain: Protoform,
    separator: Separator,
    chain_start: usize,
    sub_situations: Vec<(Path, Extent)>,
}

struct Frame {
    kind: FrameKind,
    opener_pos: usize,
    children: Vec<Protoform>,
    child_index: Integer,
    path: Path,
    situations: Vec<(Path, Extent)>,
    pending: Option<Pending>,
}

/// The kind whose capabilities manage frame children and situations.
trait Framing {
    fn child_path(&self) -> Path;
    fn add_child(
        &mut self,
        pf: Protoform,
        start: usize,
        end: usize,
        sub_situations: Vec<(Path, Extent)>,
    );
    fn flush_pending(&mut self);
    fn body_path_for_pending(&self) -> Path;
}

impl Framing for Frame {
    fn child_path(&self) -> Path {
        let mut p = self.path.clone();
        p.push(self.child_index);
        p
    }

    fn add_child(
        &mut self,
        pf: Protoform,
        start: usize,
        end: usize,
        sub_situations: Vec<(Path, Extent)>,
    ) {
        if let Some(pending) = self.pending.take() {
            let combined = BodyAttacher::attach_body(pending.chain, pending.separator, pf);
            let child_path = self.child_path();
            self.situations.push((
                child_path,
                Extent(pending.chain_start as Integer, end as Integer),
            ));
            self.situations.extend(pending.sub_situations);
            self.situations.extend(sub_situations);
            self.children.push(combined);
            self.child_index += 1;
        } else {
            let child_path = self.child_path();
            self.situations
                .push((child_path, Extent(start as Integer, end as Integer)));
            self.situations.extend(sub_situations);
            self.children.push(pf);
            self.child_index += 1;
        }
    }

    fn flush_pending(&mut self) {
        if let Some(pending) = self.pending.take() {
            let child_path = self.child_path();
            self.situations.push((
                child_path,
                Extent(
                    pending.chain_start as Integer,
                    pending.chain_start as Integer,
                ),
            ));
            self.situations.extend(pending.sub_situations);
            self.children.push(pending.chain);
            self.child_index += 1;
        }
    }

    fn body_path_for_pending(&self) -> Path {
        match &self.pending {
            Some(pending) => {
                let depth = BodyAttacher::chain_depth(&pending.chain);
                let mut path = self.child_path();
                path.extend(std::iter::repeat_n(0, depth + 1));
                path
            }
            None => self.child_path(),
        }
    }
}

// ---------------------------------------------------------------------------
// Delineator and its traits
// ---------------------------------------------------------------------------

struct Delineator<'a> {
    source: &'a str,
    pos: usize,
    stack: Vec<Frame>,
}

/// The kind whose capabilities traverse text character by character.
trait Traversing {
    fn remaining(&self) -> &str;
    fn peek(&self) -> Option<char>;
    fn advance(&mut self) -> Option<char>;
    fn skip_whitespace_and_comments(&mut self);
    fn is_structural(c: char) -> bool;
}

impl Traversing for Delineator<'_> {
    fn remaining(&self) -> &str {
        &self.source[self.pos..]
    }

    fn peek(&self) -> Option<char> {
        self.remaining().chars().next()
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.remaining().chars().next()?;
        self.pos += c.len_utf8();
        Some(c)
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            let before = self.pos;
            while let Some(c) = self.peek() {
                if c.is_whitespace() {
                    self.advance();
                } else {
                    break;
                }
            }
            if self.peek() == Some(';') {
                while let Some(c) = self.advance() {
                    if c == '\n' {
                        break;
                    }
                }
                continue;
            }
            if self.pos == before {
                break;
            }
        }
    }

    fn is_structural(c: char) -> bool {
        c.is_whitespace()
            || Enclosure::from_opener(c).is_some()
            || Enclosure::from_closer(c).is_some()
            || Boundary::from_opener(c).is_some()
            || Boundary::from_closer(c).is_some()
            || c == ';'
    }
}

/// The kind whose capability reads text into a delineation.
trait Delineating {
    fn delineate(&mut self) -> Result<Delineation, Fault>;
}

impl Delineating for Delineator<'_> {
    fn delineate(&mut self) -> Result<Delineation, Fault> {
        self.stack.push(Frame {
            kind: FrameKind::Root,
            opener_pos: 0,
            children: Vec::new(),
            child_index: 0,
            path: vec![],
            situations: Vec::new(),
            pending: None,
        });

        loop {
            self.skip_whitespace_and_comments();

            if self.pos >= self.source.len() {
                let top = self.stack.last_mut().unwrap();
                top.flush_pending();
                if self.stack.len() > 1 {
                    let frame = self.stack.last().unwrap();
                    let problem = match &frame.kind {
                        FrameKind::Root => unreachable!(),
                        FrameKind::Enclosure(enc) => Problem::Unclosed(*enc),
                        FrameKind::QualifiedAngle(_) => Problem::Unclosed(Enclosure::Angled),
                    };
                    return Err(Fault {
                        extent: Extent(frame.opener_pos as Integer, self.source.len() as Integer),
                        problem,
                    });
                }
                break;
            }

            let c = self.peek().unwrap();

            // Closers
            if let Some(enc) = Enclosure::from_closer(c) {
                let top_matches = match &self.stack.last().unwrap().kind {
                    FrameKind::Enclosure(e) => *e == enc,
                    FrameKind::QualifiedAngle(_) => enc == Enclosure::Angled,
                    FrameKind::Root => false,
                };
                if top_matches {
                    self.advance();
                    self.pop_frame()?;
                    continue;
                }
                let start = self.pos as Integer;
                self.advance();
                return Err(Fault {
                    extent: Extent(start, self.pos as Integer),
                    problem: Problem::Unopened,
                });
            }
            if Boundary::from_closer(c).is_some() {
                let start = self.pos as Integer;
                self.advance();
                return Err(Fault {
                    extent: Extent(start, self.pos as Integer),
                    problem: Problem::Unopened,
                });
            }

            // Enclosure openers (non-angle at top level, or standalone angle)
            if let Some(enc) = Enclosure::from_opener(c) {
                if self.stack.len() >= DEPTH_LIMIT {
                    return Err(Fault {
                        extent: Extent(self.pos as Integer, self.pos as Integer + 1),
                        problem: Problem::Unclosed(enc),
                    });
                }
                let opener_pos = self.pos;
                self.advance();
                let frame_path = self.stack.last().unwrap().body_path_for_pending();
                self.stack.push(Frame {
                    kind: FrameKind::Enclosure(enc),
                    opener_pos,
                    children: Vec::new(),
                    child_index: 0,
                    path: frame_path,
                    situations: Vec::new(),
                    pending: None,
                });
                continue;
            }

            // Boundary openers
            if let Some(bnd) = Boundary::from_opener(c) {
                let (pf, start) = match bnd {
                    Boundary::CurlyQuotes => self.read_curly_quotes()?,
                    Boundary::Parentheses => self.read_parentheses()?,
                };
                let end = self.pos;
                let parent = self.stack.last_mut().unwrap();
                parent.add_child(pf, start, end, vec![]);
                continue;
            }

            // Bare run
            self.handle_bare_run()?;
        }

        let root = self.stack.pop().unwrap();
        let mut situation = Situation::new();
        for (path, extent) in root.situations {
            situation.insert(path, extent);
        }
        Ok(Delineation {
            protoforms: root.children,
            situation,
        })
    }
}

// ---------------------------------------------------------------------------
// Boundary reading
// ---------------------------------------------------------------------------

/// The kind whose capabilities read opaque boundary content.
trait BoundaryReading {
    fn read_curly_quotes(&mut self) -> Result<(Protoform, usize), Fault>;
    fn read_parentheses(&mut self) -> Result<(Protoform, usize), Fault>;
}

impl BoundaryReading for Delineator<'_> {
    fn read_curly_quotes(&mut self) -> Result<(Protoform, usize), Fault> {
        let start = self.pos;
        self.advance();
        let content_start = self.pos;
        let closer = Boundary::CurlyQuotes.closer();
        loop {
            match self.advance() {
                Some(c) if c == closer => break,
                Some(_) => continue,
                None => {
                    return Err(Fault {
                        extent: Extent(start as Integer, self.source.len() as Integer),
                        problem: Problem::UnclosedBoundary(Boundary::CurlyQuotes),
                    });
                }
            }
        }
        let content = self.source[content_start..self.pos - closer.len_utf8()].to_owned();
        Ok((Protoform::Opaque(Boundary::CurlyQuotes, content), start))
    }

    fn read_parentheses(&mut self) -> Result<(Protoform, usize), Fault> {
        let start = self.pos;
        self.advance();
        let mut content = String::new();
        let mut depth = 1u32;
        let opener = Boundary::Parentheses.opener();
        let closer = Boundary::Parentheses.closer();
        loop {
            match self.advance() {
                Some('\\') => match self.advance() {
                    Some(c) if c == opener || c == closer || c == '\\' => {
                        content.push(c);
                    }
                    Some(c) => {
                        content.push('\\');
                        content.push(c);
                    }
                    None => {
                        return Err(Fault {
                            extent: Extent(start as Integer, self.source.len() as Integer),
                            problem: Problem::UnclosedBoundary(Boundary::Parentheses),
                        });
                    }
                },
                Some(c) if c == opener => {
                    depth += 1;
                    content.push(c);
                }
                Some(c) if c == closer => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                    content.push(c);
                }
                Some(c) => content.push(c),
                None => {
                    return Err(Fault {
                        extent: Extent(start as Integer, self.source.len() as Integer),
                        problem: Problem::UnclosedBoundary(Boundary::Parentheses),
                    });
                }
            }
        }
        Ok((Protoform::Opaque(Boundary::Parentheses, content), start))
    }
}

// ---------------------------------------------------------------------------
// Bare run handling
// ---------------------------------------------------------------------------

/// The kind whose capabilities handle bare runs and qualified heads.
trait BareRunHandling {
    fn handle_bare_run(&mut self) -> Result<(), Fault>;
    fn handle_bare_then_angle(&mut self, run: &str, run_start: usize) -> Result<(), Fault>;
    fn pop_frame(&mut self) -> Result<(), Fault>;
    fn handle_post_qualified(
        &mut self,
        pf: Protoform,
        chain_start: usize,
        sub_situations: Vec<(Path, Extent)>,
    ) -> Result<(), Fault>;
    fn set_or_combine_pending(
        &mut self,
        chain: Protoform,
        sep: Separator,
        chain_start: usize,
        sub_situations: Vec<(Path, Extent)>,
    );
}

impl BareRunHandling for Delineator<'_> {
    fn handle_bare_run(&mut self) -> Result<(), Fault> {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if Self::is_structural(c) {
                break;
            }
            self.advance();
        }
        let run = self.source[start..self.pos].to_owned();
        if run.is_empty() {
            return Ok(());
        }

        // Check if ends with separator + opener follows (before qualified head check)
        let chars_vec: Vec<(usize, char)> = run.char_indices().collect();
        if let Some(&(last_byte, last_char)) = chars_vec.last() {
            if let Some(trailing_sep) = Separator::identify(last_char) {
                if let Some(next_c) = self.peek() {
                    let is_opener = Enclosure::from_opener(next_c).is_some()
                        || Boundary::from_opener(next_c).is_some();
                    if is_opener {
                        let head_str = &run[..last_byte];
                        if head_str.is_empty() {
                            // Lone separator before opener: stays bare
                            let parent = self.stack.last_mut().unwrap();
                            let base_path = parent.child_path();
                            let (pf, subs) = BareRunParser::parse_bare_run(&run, start, &base_path);
                            parent.add_child(pf, start, self.pos, subs);
                            return Ok(());
                        }
                        let parent = self.stack.last_mut().unwrap();
                        let base_path = parent.child_path();
                        let (head_chain, head_subs) =
                            BareRunParser::parse_bare_run(head_str, start, &base_path);
                        self.set_or_combine_pending(head_chain, trailing_sep, start, head_subs);

                        // Push frame for body
                        if let Some(enc) = Enclosure::from_opener(next_c) {
                            if self.stack.len() >= DEPTH_LIMIT {
                                return Err(Fault {
                                    extent: Extent(self.pos as Integer, self.pos as Integer + 1),
                                    problem: Problem::Unclosed(enc),
                                });
                            }
                            let opener_pos = self.pos;
                            self.advance();
                            let frame_path = self.stack.last().unwrap().body_path_for_pending();
                            self.stack.push(Frame {
                                kind: FrameKind::Enclosure(enc),
                                opener_pos,
                                children: Vec::new(),
                                child_index: 0,
                                path: frame_path,
                                situations: Vec::new(),
                                pending: None,
                            });
                            return Ok(());
                        }
                        if let Some(bnd) = Boundary::from_opener(next_c) {
                            let (body, body_start) = match bnd {
                                Boundary::CurlyQuotes => self.read_curly_quotes()?,
                                Boundary::Parentheses => self.read_parentheses()?,
                            };
                            let end = self.pos;
                            let parent = self.stack.last_mut().unwrap();
                            parent.add_child(body, body_start, end, vec![]);
                            return Ok(());
                        }
                    }
                }
            }
        }

        // Check if followed by angle opener (qualified head)
        // Only when the run does NOT end with a separator
        if self.peek() == Some(Enclosure::Angled.opener()) {
            return self.handle_bare_then_angle(&run, start);
        }

        // Plain bare run
        let end = self.pos;
        let parent = self.stack.last_mut().unwrap();
        let base_path = parent.child_path();
        let (pf, subs) = BareRunParser::parse_bare_run(&run, start, &base_path);
        parent.add_child(pf, start, end, subs);
        Ok(())
    }

    fn handle_bare_then_angle(&mut self, run: &str, run_start: usize) -> Result<(), Fault> {
        let split_points = BareRunParser::find_split_points(run);

        let (prefix_chain_data, symbol) = if split_points.is_empty() {
            (None, run.to_owned())
        } else {
            let last = &split_points[split_points.len() - 1];
            let sep_len = last.separator.glyph().len_utf8();
            let symbol_start = last.byte_offset + sep_len;
            let symbol = run[symbol_start..].to_owned();
            let prefix_str = &run[..last.byte_offset];
            let parent = self.stack.last().unwrap();
            let base_path = parent.child_path();
            let (prefix, prefix_subs) =
                BareRunParser::parse_bare_run(prefix_str, run_start, &base_path);
            (Some((prefix, last.separator, prefix_subs)), symbol)
        };

        if let Some((prefix, sep, subs)) = prefix_chain_data {
            self.set_or_combine_pending(prefix, sep, run_start, subs);
        }

        if self.stack.len() >= DEPTH_LIMIT {
            return Err(Fault {
                extent: Extent(self.pos as Integer, self.pos as Integer + 1),
                problem: Problem::Unclosed(Enclosure::Angled),
            });
        }
        self.advance(); // consume '<'
        let frame_path = self.stack.last().unwrap().body_path_for_pending();
        self.stack.push(Frame {
            kind: FrameKind::QualifiedAngle(symbol),
            opener_pos: run_start,
            children: Vec::new(),
            child_index: 0,
            path: frame_path,
            situations: Vec::new(),
            pending: None,
        });
        Ok(())
    }

    fn pop_frame(&mut self) -> Result<(), Fault> {
        let mut frame = self.stack.pop().unwrap();
        frame.flush_pending();

        match frame.kind {
            FrameKind::Root => unreachable!(),
            FrameKind::Enclosure(enc) => {
                let enclosed = Protoform::Enclosed(enc, frame.children);
                let start = frame.opener_pos;
                let end = self.pos;
                let parent = self.stack.last_mut().unwrap();
                parent.situations.extend(frame.situations);
                parent.add_child(enclosed, start, end, vec![]);
            }
            FrameKind::QualifiedAngle(symbol) => {
                let qualified_head = Head::Qualified(symbol, frame.children);
                let qualified_pf = Protoform::Bare(qualified_head);
                let chain_start = frame.opener_pos;
                let parent = self.stack.last_mut().unwrap();
                parent.situations.extend(frame.situations);

                let (current_pf, sub_subs) = if let Some(pending) = parent.pending.take() {
                    let combined =
                        BodyAttacher::attach_body(pending.chain, pending.separator, qualified_pf);
                    (combined, pending.sub_situations)
                } else {
                    (qualified_pf, vec![])
                };

                self.handle_post_qualified(current_pf, chain_start, sub_subs)?;
            }
        }
        Ok(())
    }

    fn handle_post_qualified(
        &mut self,
        current_pf: Protoform,
        chain_start: usize,
        sub_situations: Vec<(Path, Extent)>,
    ) -> Result<(), Fault> {
        if let Some(c) = self.peek() {
            if let Some(sep) = Separator::identify(c) {
                let sep_pos = self.pos;
                self.advance();
                if let Some(next_c) = self.peek() {
                    let can_be_body = !next_c.is_whitespace()
                        && Enclosure::from_closer(next_c).is_none()
                        && Boundary::from_closer(next_c).is_none()
                        && next_c != ';';
                    if can_be_body {
                        self.set_or_combine_pending(current_pf, sep, chain_start, sub_situations);

                        if let Some(enc) = Enclosure::from_opener(next_c) {
                            if self.stack.len() >= DEPTH_LIMIT {
                                return Err(Fault {
                                    extent: Extent(self.pos as Integer, self.pos as Integer + 1),
                                    problem: Problem::Unclosed(enc),
                                });
                            }
                            let opener_pos = self.pos;
                            self.advance();
                            let frame_path = self.stack.last().unwrap().body_path_for_pending();
                            self.stack.push(Frame {
                                kind: FrameKind::Enclosure(enc),
                                opener_pos,
                                children: Vec::new(),
                                child_index: 0,
                                path: frame_path,
                                situations: Vec::new(),
                                pending: None,
                            });
                            return Ok(());
                        }
                        if let Some(bnd) = Boundary::from_opener(next_c) {
                            let (body, body_start) = match bnd {
                                Boundary::CurlyQuotes => self.read_curly_quotes()?,
                                Boundary::Parentheses => self.read_parentheses()?,
                            };
                            let end = self.pos;
                            let parent = self.stack.last_mut().unwrap();
                            parent.add_child(body, body_start, end, vec![]);
                            return Ok(());
                        }
                        // Body is a bare word: pending set, main loop reads it
                        return Ok(());
                    }
                }
                // No valid body: un-advance
                self.pos = sep_pos;
            }
        }
        // Add as standalone
        let end = self.pos;
        let parent = self.stack.last_mut().unwrap();
        parent.add_child(current_pf, chain_start, end, sub_situations);
        Ok(())
    }

    fn set_or_combine_pending(
        &mut self,
        chain: Protoform,
        sep: Separator,
        chain_start: usize,
        sub_situations: Vec<(Path, Extent)>,
    ) {
        let parent = self.stack.last_mut().unwrap();
        if let Some(existing) = parent.pending.take() {
            let combined = BodyAttacher::attach_body(existing.chain, existing.separator, chain);
            let mut merged_subs = existing.sub_situations;
            merged_subs.extend(sub_situations);
            parent.pending = Some(Pending {
                chain: combined,
                separator: sep,
                chain_start: existing.chain_start,
                sub_situations: merged_subs,
            });
        } else {
            parent.pending = Some(Pending {
                chain,
                separator: sep,
                chain_start,
                sub_situations,
            });
        }
    }
}

// ---------------------------------------------------------------------------
// Protosizable for Text: the delineation entry point
// ---------------------------------------------------------------------------

impl Protosizable for Text {
    type Fault = Fault;

    fn protosize(&self) -> Result<Delineation, Fault> {
        let mut d = Delineator {
            source: self,
            pos: 0,
            stack: Vec::new(),
        };
        d.delineate()
    }
}

impl<T, C> Protosizable for crate::Potential<T, C> {
    type Fault = Fault;

    fn protosize(&self) -> Result<Delineation, Fault> {
        use crate::Texted as _;
        <Text as Protosizable>::protosize(&self.text().to_owned())
    }
}
