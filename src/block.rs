use std::ops::Range;

use crate::{BlockScanning, Headed, Realize, Shape, StringCarrying, Textualize, WalkFault};

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
    pub body: StringCarrier,
    pub span: Range<usize>,
}

/// The first-pass scanner. It makes blocks and does not assign dialect meaning.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BlockScanner;

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
        match &self.body {
            StringCarrier::Bare(body) => text.push_str(body),
            StringCarrier::Parenthesized(body) => {
                text.push('(');
                text.push_str(body);
                text.push(')');
            }
            StringCarrier::CurlyQuoted(body) => {
                text.push('“');
                text.push_str(body);
                text.push('”');
            }
        }
        SourceText(text)
    }
}

impl Textualize for StringCarrier {
    type Textual = SourceText;

    fn textualize(&self) -> SourceText {
        SourceText(self.textual_body().to_owned())
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
        BlockScanner.scan(&self.0)
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
    fn scan(&self, source: &str) -> Result<Vec<Block>, WalkFault>;
}

impl Scanning for BlockScanner {
    fn scan(&self, source: &str) -> Result<Vec<Block>, WalkFault> {
        let characters: Vec<char> = source.chars().collect();
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
                && !matches!(characters[index], '(' | '“' | ')' | '”')
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
                    if !prefix.is_empty() && !dotted {
                        return Err(WalkFault::InvalidHead);
                    }
                    index += 1;
                    let (body, next) = self.parenthesized(&characters, index)?;
                    blocks.push(Block {
                        head,
                        shape: if dotted {
                            Shape::DottedParenthesized
                        } else {
                            Shape::Parenthesized
                        },
                        body: StringCarrier::Parenthesized(body),
                        span: start..next,
                    });
                    index = next;
                }
                Some('“') => {
                    if !prefix.is_empty() && !dotted {
                        return Err(WalkFault::InvalidHead);
                    }
                    index += 1;
                    let (body, next) = self.curly_quoted(&characters, index)?;
                    blocks.push(Block {
                        head,
                        shape: if dotted {
                            Shape::DottedCurlyQuoted
                        } else {
                            Shape::CurlyQuoted
                        },
                        body: StringCarrier::CurlyQuoted(body),
                        span: start..next,
                    });
                    index = next;
                }
                Some(')') | Some('”') => {
                    return Err(WalkFault::UnexpectedCloser(characters[index]));
                }
                Some(_) | None => {
                    if prefix.is_empty() || dotted {
                        return Err(WalkFault::InvalidHead);
                    }
                    blocks.push(Block {
                        head: None,
                        shape: Shape::Bare,
                        body: StringCarrier::Bare(prefix),
                        span: start..index,
                    });
                }
            }
        }
        Ok(blocks)
    }
}

trait StringScanning {
    fn parenthesized(
        &self,
        characters: &[char],
        start: usize,
    ) -> Result<(String, usize), WalkFault>;
    fn curly_quoted(&self, characters: &[char], start: usize)
    -> Result<(String, usize), WalkFault>;
}

impl StringScanning for BlockScanner {
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
}
