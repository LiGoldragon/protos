use std::ops::Range;

use crate::{
    BlockScanning, Headed, Realize, Shape, SourceSlicing, StringCarrying, Textualize, WalkFault,
};

/// The dotted prefix that is structurally part of a block. An absent value is
/// an unprefixed block.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Head(pub String);

/// Text as data before a dialect gives it a real type.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceText(pub String);

/// The string forms whose lexical opacity is universally known.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StringCarrier {
    Bare(String),
    Parenthesized(String),
    CurlyQuoted(String),
}

/// One lexical unit found by the first pass. Its body remains textual.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Block {
    pub head: Option<Head>,
    pub shape: Shape,
    pub body: SourceText,
    pub string_carrier: Option<StringCarrier>,
    pub span: Range<usize>,
}

/// The first-pass scanner. It retains the source it separates and does not
/// assign dialect meaning.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlockScanner {
    source: SourceText,
}

impl Headed for Block {
    fn head(&self) -> Option<&Head> {
        self.head.as_ref()
    }
}

impl Textualize for Block {
    type Textual = SourceText;

    fn textualize(&self) -> SourceText {
        let mut text = String::new();
        if let Some(head) = &self.head {
            text.push_str(&head.0);
            text.push('.');
        }
        let (opening, closing) = match self.shape {
            Shape::Bare => (None, None),
            Shape::CurlyQuoted | Shape::DottedCurlyQuoted => (Some('“'), Some('”')),
            Shape::Parenthesized | Shape::DottedParenthesized => (Some('('), Some(')')),
            Shape::SquareBracketed | Shape::DottedSquareBracketed => (Some('['), Some(']')),
            Shape::Braced | Shape::DottedBraced => (Some('{'), Some('}')),
        };
        if let Some(opening) = opening {
            text.push(opening);
        }
        text.push_str(&self.body.0);
        if let Some(closing) = closing {
            text.push(closing);
        }
        SourceText(text)
    }
}

impl Textualize for StringCarrier {
    type Textual = SourceText;

    fn textualize(&self) -> SourceText {
        match self {
            Self::Bare(body) => SourceText(body.clone()),
            Self::Parenthesized(body) => SourceText(format!("({body})")),
            Self::CurlyQuoted(body) => SourceText(format!("“{body}”")),
        }
    }
}

impl StringCarrying for StringCarrier {
    fn textual_body(&self) -> &str {
        match self {
            Self::Bare(body) | Self::Parenthesized(body) | Self::CurlyQuoted(body) => body,
        }
    }
}

impl BlockScanning for SourceText {
    fn blocks(&self) -> Result<Vec<Block>, WalkFault> {
        BlockScanner {
            source: self.clone(),
        }
        .scan()
    }
}

impl SourceSlicing for SourceText {
    fn source_slice(&self, span: Range<usize>) -> Option<&str> {
        self.0.get(span)
    }
}

impl Realize for SourceText {
    type Real = Vec<Block>;
    type Fault = WalkFault;

    fn realize(&self) -> Result<Vec<Block>, WalkFault> {
        self.blocks()
    }
}

trait Scanning {
    fn scan(&self) -> Result<Vec<Block>, WalkFault>;
}

impl Scanning for BlockScanner {
    fn scan(&self) -> Result<Vec<Block>, WalkFault> {
        let source = &self.source.0;
        let characters: Vec<char> = source.chars().collect();
        let byte_offsets: Vec<usize> = source
            .char_indices()
            .map(|(offset, _)| offset)
            .chain(std::iter::once(source.len()))
            .collect();
        let mut blocks = Vec::new();
        let mut index = 0;

        while index < characters.len() {
            while index < characters.len() && characters[index].is_whitespace() {
                index += 1;
            }
            if index == characters.len() {
                break;
            }

            let start = index;
            while index < characters.len()
                && !characters[index].is_whitespace()
                && !matches!(
                    characters[index],
                    '(' | ')' | '“' | '”' | '[' | ']' | '{' | '}'
                )
            {
                index += 1;
            }
            let prefix: String = characters[start..index].iter().collect();
            let dotted = prefix.ends_with('.');
            let head = if dotted {
                let raw = &prefix[..prefix.len() - 1];
                if raw.is_empty() {
                    return Err(WalkFault::InvalidHead);
                }
                Some(Head(raw.to_owned()))
            } else {
                None
            };

            match characters.get(index).copied() {
                Some('(') => {
                    self.require_delimited_prefix(&prefix, dotted)?;
                    index += 1;
                    let (body, next) = self.parenthesized(&characters, index)?;
                    blocks.push(Block {
                        head,
                        shape: if dotted {
                            Shape::DottedParenthesized
                        } else {
                            Shape::Parenthesized
                        },
                        body: SourceText(body.clone()),
                        string_carrier: Some(StringCarrier::Parenthesized(body)),
                        span: byte_offsets[start]..byte_offsets[next],
                    });
                    index = next;
                }
                Some('“') => {
                    self.require_delimited_prefix(&prefix, dotted)?;
                    index += 1;
                    let (body, next) = self.curly_quoted(&characters, index)?;
                    blocks.push(Block {
                        head,
                        shape: if dotted {
                            Shape::DottedCurlyQuoted
                        } else {
                            Shape::CurlyQuoted
                        },
                        body: SourceText(body.clone()),
                        string_carrier: Some(StringCarrier::CurlyQuoted(body)),
                        span: byte_offsets[start]..byte_offsets[next],
                    });
                    index = next;
                }
                Some('[') => {
                    self.require_delimited_prefix(&prefix, dotted)?;
                    index += 1;
                    let (body, next) =
                        self.structural(&characters, index, '[', ']', Shape::SquareBracketed)?;
                    blocks.push(Block {
                        head,
                        shape: if dotted {
                            Shape::DottedSquareBracketed
                        } else {
                            Shape::SquareBracketed
                        },
                        body: SourceText(body),
                        string_carrier: None,
                        span: byte_offsets[start]..byte_offsets[next],
                    });
                    index = next;
                }
                Some('{') => {
                    self.require_delimited_prefix(&prefix, dotted)?;
                    index += 1;
                    let (body, next) =
                        self.structural(&characters, index, '{', '}', Shape::Braced)?;
                    blocks.push(Block {
                        head,
                        shape: if dotted {
                            Shape::DottedBraced
                        } else {
                            Shape::Braced
                        },
                        body: SourceText(body),
                        string_carrier: None,
                        span: byte_offsets[start]..byte_offsets[next],
                    });
                    index = next;
                }
                Some(')') | Some('”') | Some(']') | Some('}') => {
                    return Err(WalkFault::UnexpectedCloser(characters[index]));
                }
                Some(_) | None => {
                    if prefix.is_empty() || dotted {
                        return Err(WalkFault::InvalidHead);
                    }
                    blocks.push(Block {
                        head: None,
                        shape: Shape::Bare,
                        body: SourceText(prefix.clone()),
                        string_carrier: Some(StringCarrier::Bare(prefix)),
                        span: byte_offsets[start]..byte_offsets[index],
                    });
                }
            }
        }
        Ok(blocks)
    }
}

trait PrefixChecking {
    fn require_delimited_prefix(&self, prefix: &str, dotted: bool) -> Result<(), WalkFault>;
}

impl PrefixChecking for BlockScanner {
    fn require_delimited_prefix(&self, prefix: &str, dotted: bool) -> Result<(), WalkFault> {
        if !prefix.is_empty() && !dotted {
            return Err(WalkFault::InvalidHead);
        }
        Ok(())
    }
}

trait DelimiterScanning {
    fn parenthesized(
        &self,
        characters: &[char],
        start: usize,
    ) -> Result<(String, usize), WalkFault>;
    fn curly_quoted(&self, characters: &[char], start: usize)
    -> Result<(String, usize), WalkFault>;
    fn structural(
        &self,
        characters: &[char],
        start: usize,
        opening: char,
        closing: char,
        shape: Shape,
    ) -> Result<(String, usize), WalkFault>;
}

impl DelimiterScanning for BlockScanner {
    fn parenthesized(
        &self,
        characters: &[char],
        start: usize,
    ) -> Result<(String, usize), WalkFault> {
        let mut depth = 0;
        let mut body = String::new();
        let mut index = start;
        while index < characters.len() {
            match characters[index] {
                '\\' if index + 1 < characters.len() => {
                    body.push(characters[index]);
                    index += 1;
                    body.push(characters[index]);
                }
                '(' => {
                    depth += 1;
                    body.push('(');
                }
                ')' if depth == 0 => return Ok((body, index + 1)),
                ')' => {
                    depth -= 1;
                    body.push(')');
                }
                character => body.push(character),
            }
            index += 1;
        }
        Err(WalkFault::UnclosedBlock(Shape::Parenthesized))
    }

    fn curly_quoted(
        &self,
        characters: &[char],
        start: usize,
    ) -> Result<(String, usize), WalkFault> {
        let mut body = String::new();
        let mut index = start;
        while index < characters.len() {
            match characters[index] {
                '\\' if index + 1 < characters.len() => {
                    body.push(characters[index]);
                    index += 1;
                    body.push(characters[index]);
                }
                '”' => return Ok((body, index + 1)),
                character => body.push(character),
            }
            index += 1;
        }
        Err(WalkFault::UnclosedBlock(Shape::CurlyQuoted))
    }

    fn structural(
        &self,
        characters: &[char],
        start: usize,
        opening: char,
        closing: char,
        shape: Shape,
    ) -> Result<(String, usize), WalkFault> {
        let mut depth = 0;
        let mut body = String::new();
        let mut index = start;
        while index < characters.len() {
            match characters[index] {
                '\\' if index + 1 < characters.len() => {
                    body.push(characters[index]);
                    index += 1;
                    body.push(characters[index]);
                }
                '“' => {
                    let quote_start = index + 1;
                    let (quoted, next) = self.curly_quoted(characters, quote_start)?;
                    body.push('“');
                    body.push_str(&quoted);
                    body.push('”');
                    index = next - 1;
                }
                '(' => {
                    let parenthesis_start = index + 1;
                    let (parenthesized, next) =
                        self.parenthesized(characters, parenthesis_start)?;
                    body.push('(');
                    body.push_str(&parenthesized);
                    body.push(')');
                    index = next - 1;
                }
                character if character == opening => {
                    depth += 1;
                    body.push(character);
                }
                character if character == closing && depth == 0 => return Ok((body, index + 1)),
                character if character == closing => {
                    depth -= 1;
                    body.push(character);
                }
                character => body.push(character),
            }
            index += 1;
        }
        Err(WalkFault::UnclosedBlock(shape))
    }
}
