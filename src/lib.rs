//! The universal structural boundary for Protos dialects.
//!
//! A dialect receives `Portion` values and supplies its own type anatomy. This
//! crate owns the sole character reader and, in the next slice, the sole
//! character writer.

use std::fmt;

pub struct Text<T = ()> {
    normalized: String,
    content_hash: ContentHash,
    delineation: Option<Delineation>,
    target: std::marker::PhantomData<fn() -> T>,
}

pub struct ContentHash(u64);

pub struct Symbol(String);

pub struct Extent {
    pub start: usize,
    pub end: usize,
}

pub enum Separator {
    Period,
    Exclamation,
    Colon,
}

/// The anatomy a dialect expects when it considers emitting text unquoted.
pub enum BareExpectation {
    Symbol,
    String,
}

pub enum Enclosure {
    Braced,
    Bracketed,
    Guillemets,
    Angled,
    CurlyQuote,
}

pub enum StructuralEnclosure {
    Braced,
    Bracketed,
    Guillemets,
    Angled,
}

/// A dialect-owned delimiter recognized by the common reader without becoming
/// one of Protos's five universal enclosures.
pub enum OpaqueBoundary {
    CurlyQuote,
    Dialect(DialectBoundary),
}

pub enum Boundary {
    Structural(StructuralEnclosure),
    Opaque(OpaqueBoundary),
}

pub enum DialectBoundary {
    Parentheses,
}

/// One structural value. Its variant is its inline anatomy and carries the
/// value's one half-open UTF-8 byte extent.
#[non_exhaustive]
pub enum Portion {
    Headed(Extent, Headed),
    Enclosed(Extent, Enclosed),
    Bare(Extent, Bare),
}

pub struct Headed {
    pub head: Symbol,
    pub separator: Separator,
    pub body: Box<Portion>,
}

pub enum Enclosed {
    Structural(StructuralEnclosed),
    Opaque(OpaqueEnclosed),
}

pub struct StructuralEnclosed {
    enclosure: StructuralEnclosure,
    portions: Vec<Portion>,
}

pub struct OpaqueEnclosed {
    boundary: OpaqueBoundary,
    content: String,
}

pub struct Bare {
    pub symbol: Symbol,
}

pub struct Delineation {
    pub portions: Vec<Portion>,
}

pub type Prospective<T> = Text<T>;

pub struct Fault {
    pub extent: Extent,
    pub problem: FaultProblem,
}

pub enum FaultProblem {
    UnexpectedCloser,
    UnclosedDelimiter,
    MissingHead,
    MissingBody,
    ExpectedOnePortion,
    ExpectedShape,
    ExpectedBareSymbol,
    InvalidSignedInteger,
    IntegerOutOfRange,
    InvalidDecimal,
    NonFiniteDecimal,
}

pub enum Layout {
    Flat,
}

pub trait Delineatable {
    type Delineation;

    fn delineate(&self) -> Result<Self::Delineation, Fault>;
}

pub trait Embodiable {
    type Embodied: Embodied;

    fn embody(&self) -> Result<Self::Embodied, Fault>;
}

/// A final Rust type owns its inbound Portion anatomy.
pub trait Embodied: Sized {
    fn from_portion(portion: &Portion) -> Result<Self, Fault>;
}

/// A final Rust type owns its outbound Portion anatomy; Protos prints it.
pub trait Textualizable: Embodied {
    fn to_portion(&self) -> Portion;

    fn textualize(&self) -> Text {
        self.to_portion().print(Layout::Flat)
    }
}

/// A shape predicate used by a dialect to select an anatomy, never a parser.
pub trait ShapeDefined: Embodied {
    fn matches(portion: &Portion) -> bool;
}

pub trait ContentHashable {
    fn content_hash(&self) -> ContentHash;
}

/// The universal anatomy question a dialect asks before it chooses Bare.
pub trait BareSafe {
    fn is_bare_safe_for(&self, expectation: BareExpectation) -> bool;
}

/// Canonical text recovered from one already-delineated Portion.
pub trait PortionText {
    fn canonical_text(&self) -> Text;
}

/// Universal scalar questions over anatomy already produced by the delineator.
pub trait ScalarAnatomy {
    fn signed_i64(&self) -> Result<i64, Fault>;
    fn decimal_f64(&self) -> Result<f64, Fault>;
}

pub trait EnclosedArity {
    fn arity(&self) -> usize;
}

pub trait EnclosedAnatomy {
    fn structural_enclosure(&self) -> Option<StructuralEnclosure>;
    fn opaque_boundary(&self) -> Option<OpaqueBoundary>;
    fn portions(&self) -> Option<&[Portion]>;
    fn opaque_content(&self) -> Option<&str>;
}

/// The only Protos capability which writes structural characters.
pub trait Printing {
    fn print(&self, layout: Layout) -> Text;
}

/// Lets callers retain the extents computed by the writer without re-reading
/// the text it just wrote.
pub trait DelineatedText {
    fn delineation(&self) -> Option<&Delineation>;
    fn retag<U>(self) -> Text<U>
    where
        Self: Sized;
}

impl<T> From<&str> for Text<T> {
    fn from(value: &str) -> Self {
        let mut parser = Parser {
            input: value,
            cursor: 0,
        };
        let normalized = match parser.delineate_document() {
            Ok(delineation) => {
                let mut printer = Printer {
                    output: String::new(),
                };
                printer.delineation(&delineation, Layout::Flat);
                printer.output
            }
            Err(_) => value.to_owned(),
        };
        let content_hash = TextHasher.hash(&normalized);
        Self {
            normalized,
            content_hash,
            delineation: None,
            target: std::marker::PhantomData,
        }
    }
}

impl<T> From<String> for Text<T> {
    fn from(value: String) -> Self {
        Self::from(value.as_str())
    }
}

impl<T> AsRef<str> for Text<T> {
    fn as_ref(&self) -> &str {
        &self.normalized
    }
}

impl AsRef<str> for Symbol {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl TryFrom<&str> for Symbol {
    type Error = Fault;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let text = Text::<()>::from(value);
        let delineation = text.delineate()?;
        match delineation.portions.as_slice() {
            [Portion::Bare(extent, bare)]
                if extent.start == 0 && extent.end == text.as_ref().len() =>
            {
                Ok(Symbol(bare.symbol.0.to_owned()))
            }
            _ => Err(Fault {
                extent: Extent {
                    start: 0,
                    end: text.as_ref().len(),
                },
                problem: FaultProblem::ExpectedBareSymbol,
            }),
        }
    }
}

impl AsRef<Extent> for Portion {
    fn as_ref(&self) -> &Extent {
        match self {
            Self::Headed(extent, _) | Self::Enclosed(extent, _) | Self::Bare(extent, _) => extent,
        }
    }
}

impl EnclosedArity for Enclosed {
    fn arity(&self) -> usize {
        match self {
            Self::Structural(enclosed) => enclosed.portions.len(),
            Self::Opaque(_) => 0,
        }
    }
}

impl EnclosedAnatomy for Enclosed {
    fn structural_enclosure(&self) -> Option<StructuralEnclosure> {
        match self {
            Self::Structural(enclosed) => Some(enclosed.enclosure),
            Self::Opaque(_) => None,
        }
    }

    fn opaque_boundary(&self) -> Option<OpaqueBoundary> {
        match self {
            Self::Structural(_) => None,
            Self::Opaque(enclosed) => Some(enclosed.boundary),
        }
    }

    fn portions(&self) -> Option<&[Portion]> {
        match self {
            Self::Structural(enclosed) => Some(&enclosed.portions),
            Self::Opaque(_) => None,
        }
    }

    fn opaque_content(&self) -> Option<&str> {
        match self {
            Self::Structural(_) => None,
            Self::Opaque(enclosed) => Some(&enclosed.content),
        }
    }
}

impl From<Symbol> for Bare {
    fn from(symbol: Symbol) -> Self {
        Self { symbol }
    }
}

impl From<Bare> for Portion {
    fn from(bare: Bare) -> Self {
        let provisional = Self::Bare(Extent { start: 0, end: 0 }, bare);
        let mut printer = Printer {
            output: String::new(),
        };
        printer.materialize(provisional)
    }
}

impl From<(Symbol, Separator, Portion)> for Headed {
    fn from((head, separator, body): (Symbol, Separator, Portion)) -> Self {
        Self {
            head,
            separator,
            body: Box::new(body),
        }
    }
}

impl From<Headed> for Portion {
    fn from(headed: Headed) -> Self {
        let provisional = Self::Headed(Extent { start: 0, end: 0 }, headed);
        let mut printer = Printer {
            output: String::new(),
        };
        printer.materialize(provisional)
    }
}

impl From<StructuralEnclosure> for Boundary {
    fn from(enclosure: StructuralEnclosure) -> Self {
        Self::Structural(enclosure)
    }
}

impl From<OpaqueBoundary> for Boundary {
    fn from(boundary: OpaqueBoundary) -> Self {
        Self::Opaque(boundary)
    }
}

impl From<(StructuralEnclosure, Vec<Portion>)> for StructuralEnclosed {
    fn from((enclosure, portions): (StructuralEnclosure, Vec<Portion>)) -> Self {
        Self {
            enclosure,
            portions,
        }
    }
}

impl TryFrom<(OpaqueBoundary, String)> for OpaqueEnclosed {
    type Error = Fault;

    fn try_from((boundary, content): (OpaqueBoundary, String)) -> Result<Self, Self::Error> {
        // Opaque payloads still become public Portion values only through the
        // sole writer and reader. This rejects curly-quote payloads whose
        // balance cannot be represented, while parenthetical payloads are
        // canonically escaped by the writer.
        let candidate = Self { boundary, content };
        let provisional = Portion::Enclosed(
            Extent { start: 0, end: 0 },
            Enclosed::Opaque(Self {
                boundary: candidate.boundary,
                content: candidate.content.to_owned(),
            }),
        );
        let printed = provisional.print(Layout::Flat);
        let valid = printed.delineate().is_ok_and(|delineation| {
            matches!(delineation.portions.as_slice(), [Portion::Enclosed(_, Enclosed::Opaque(parsed))]
                if parsed.boundary == candidate.boundary && parsed.content == candidate.content)
        });
        if valid {
            Ok(candidate)
        } else {
            Err(Fault {
                extent: Extent {
                    start: 0,
                    end: candidate.content.len(),
                },
                problem: FaultProblem::ExpectedShape,
            })
        }
    }
}

impl From<StructuralEnclosed> for Enclosed {
    fn from(enclosed: StructuralEnclosed) -> Self {
        Self::Structural(enclosed)
    }
}

impl From<OpaqueEnclosed> for Enclosed {
    fn from(enclosed: OpaqueEnclosed) -> Self {
        Self::Opaque(enclosed)
    }
}

impl From<Enclosed> for Portion {
    fn from(enclosed: Enclosed) -> Self {
        let provisional = Self::Enclosed(Extent { start: 0, end: 0 }, enclosed);
        let mut printer = Printer {
            output: String::new(),
        };
        printer.materialize(provisional)
    }
}

impl Portion {
    /// Materialize a canonical signed integer through the sole writer.
    pub fn from_signed_i64(value: i64) -> Self {
        Self::from(Bare::from(Symbol(value.to_string())))
    }

    /// Materialize a finite, point-mandatory decimal through the sole writer.
    pub fn from_decimal_f64(value: f64) -> Result<Self, Fault> {
        let source = canonical_decimal_f64(value)?;
        let (integer, fraction) = source
            .split_once('.')
            .expect("a canonical Protos decimal always has a point");
        Ok(Self::from(Headed::from((
            Symbol(integer.to_owned()),
            Separator::Period,
            Self::from(Bare::from(Symbol(fraction.to_owned()))),
        ))))
    }

    /// Materialize expected String content as one unquoted Portion or a
    /// validated balanced-curly opaque Portion.
    pub fn from_expected_string(content: &str) -> Result<Self, Fault> {
        let text = Text::<()>::from(content);
        if text.is_bare_safe_for(BareExpectation::String) {
            return Ok(text
                .delineate()
                .expect("bare safety proved delineation succeeds")
                .portions
                .into_iter()
                .next()
                .expect("String bare safety proved exactly one Portion"));
        }
        let opaque = OpaqueEnclosed::try_from((OpaqueBoundary::CurlyQuote, content.to_owned()))?;
        Ok(Self::from(Enclosed::from(opaque)))
    }
}

impl<T> Delineatable for Text<T> {
    type Delineation = Delineation;

    fn delineate(&self) -> Result<Self::Delineation, Fault> {
        let mut parser = Parser {
            input: self.as_ref(),
            cursor: 0,
        };
        parser.delineate_document()
    }
}

impl<T: Embodied> Embodiable for Text<T> {
    type Embodied = T;

    fn embody(&self) -> Result<Self::Embodied, Fault> {
        let delineation = self.delineate()?;
        if delineation.portions.len() != 1 {
            return Err(Fault {
                extent: Extent {
                    start: 0,
                    end: self.as_ref().len(),
                },
                problem: FaultProblem::ExpectedOnePortion,
            });
        }
        T::from_portion(&delineation.portions[0])
    }
}

impl<T> ContentHashable for Text<T> {
    fn content_hash(&self) -> ContentHash {
        ContentHash(self.content_hash.0)
    }
}

impl<T> BareSafe for Text<T> {
    fn is_bare_safe_for(&self, expectation: BareExpectation) -> bool {
        self.delineate().is_ok_and(|delineation| match expectation {
            BareExpectation::Symbol => matches!(
                delineation.portions.as_slice(),
                [Portion::Bare(extent, _)] if extent.start == 0 && extent.end == self.as_ref().len()
            ),
            BareExpectation::String => delineation.portions.len() == 1,
        })
    }
}

impl PortionText for Portion {
    fn canonical_text(&self) -> Text {
        self.print(Layout::Flat)
    }
}

impl ScalarAnatomy for Portion {
    fn signed_i64(&self) -> Result<i64, Fault> {
        let Portion::Bare(_, bare) = self else {
            return Err(scalar_fault(self, FaultProblem::InvalidSignedInteger));
        };
        let source = bare.symbol.as_ref();
        if !canonical_signed_integer(source, true) {
            return Err(scalar_fault(self, FaultProblem::InvalidSignedInteger));
        }
        source
            .parse()
            .map_err(|_| scalar_fault(self, FaultProblem::IntegerOutOfRange))
    }

    fn decimal_f64(&self) -> Result<f64, Fault> {
        let Portion::Headed(_, headed) = self else {
            return Err(scalar_fault(self, FaultProblem::InvalidDecimal));
        };
        if headed.separator != Separator::Period {
            return Err(scalar_fault(self, FaultProblem::InvalidDecimal));
        }
        let Portion::Bare(_, fraction) = headed.body.as_ref() else {
            return Err(scalar_fault(self, FaultProblem::InvalidDecimal));
        };
        let integer = headed.head.as_ref();
        let fraction = fraction.symbol.as_ref();
        if !canonical_signed_integer(integer, false)
            || fraction.is_empty()
            || !fraction.bytes().all(|byte| byte.is_ascii_digit())
        {
            return Err(scalar_fault(self, FaultProblem::InvalidDecimal));
        }
        let source = format!("{integer}.{fraction}");
        let value = source
            .parse::<f64>()
            .map_err(|_| scalar_fault(self, FaultProblem::InvalidDecimal))?;
        if value.is_finite() {
            Ok(value)
        } else {
            Err(scalar_fault(self, FaultProblem::NonFiniteDecimal))
        }
    }
}

fn scalar_fault(portion: &Portion, problem: FaultProblem) -> Fault {
    Fault {
        extent: Extent {
            start: portion.as_ref().start,
            end: portion.as_ref().end,
        },
        problem,
    }
}

fn canonical_signed_integer(source: &str, reject_negative_zero: bool) -> bool {
    let digits = source.strip_prefix('-').unwrap_or(source);
    !digits.is_empty()
        && digits.bytes().all(|byte| byte.is_ascii_digit())
        && (digits == "0" || !digits.starts_with('0'))
        && (!reject_negative_zero || source != "-0")
}

fn canonical_decimal_f64(value: f64) -> Result<String, Fault> {
    if !value.is_finite() {
        return Err(construction_fault(FaultProblem::NonFiniteDecimal));
    }
    let source = value.to_string();
    let Some(index) = source.find('e').or_else(|| source.find('E')) else {
        return Ok(if source.contains('.') {
            source
        } else {
            format!("{source}.0")
        });
    };
    let mantissa = &source[..index];
    let exponent = source[index + 1..]
        .parse::<i32>()
        .expect("Rust f64 formatting emits an integer exponent");
    let (negative, unsigned) = match mantissa.strip_prefix('-') {
        Some(unsigned) => (true, unsigned),
        None => (false, mantissa),
    };
    let (whole, fractional) = unsigned.split_once('.').unwrap_or((unsigned, ""));
    let digits = format!("{whole}{fractional}");
    let point = whole.len() as i32 + exponent;
    let mut expanded = String::new();
    if negative {
        expanded.push('-');
    }
    if point <= 0 {
        expanded.push_str("0.");
        for _ in point..0 {
            expanded.push('0');
        }
        expanded.push_str(&digits);
    } else if point as usize >= digits.len() {
        expanded.push_str(&digits);
        for _ in digits.len()..point as usize {
            expanded.push('0');
        }
        expanded.push_str(".0");
    } else {
        expanded.push_str(&digits[..point as usize]);
        expanded.push('.');
        expanded.push_str(&digits[point as usize..]);
    }
    Ok(expanded)
}

fn construction_fault(problem: FaultProblem) -> Fault {
    Fault {
        extent: Extent { start: 0, end: 0 },
        problem,
    }
}

impl<T> DelineatedText for Text<T> {
    fn delineation(&self) -> Option<&Delineation> {
        self.delineation.as_ref()
    }

    fn retag<U>(self) -> Text<U> {
        Text {
            normalized: self.normalized,
            content_hash: self.content_hash,
            delineation: self.delineation,
            target: std::marker::PhantomData,
        }
    }
}

impl Printing for Delineation {
    fn print(&self, layout: Layout) -> Text {
        let mut printer = Printer {
            output: String::new(),
        };
        let delineation = printer.delineation(self, layout);
        let content_hash = TextHasher.hash(&printer.output);
        Text {
            normalized: printer.output,
            content_hash,
            delineation: Some(delineation),
            target: std::marker::PhantomData,
        }
    }
}

impl Printing for Portion {
    fn print(&self, layout: Layout) -> Text {
        let mut printer = Printer {
            output: String::new(),
        };
        let portion = printer.portion(self, layout);
        let content_hash = TextHasher.hash(&printer.output);
        Text {
            normalized: printer.output,
            content_hash,
            delineation: Some(Delineation {
                portions: vec![portion],
            }),
            target: std::marker::PhantomData,
        }
    }
}

struct Printer {
    output: String,
}

trait Rendering {
    fn materialize(&mut self, portion: Portion) -> Portion;
    fn delineation(&mut self, delineation: &Delineation, layout: Layout) -> Delineation;
    fn portion(&mut self, portion: &Portion, layout: Layout) -> Portion;
    fn headed(&mut self, headed: &Headed, layout: Layout) -> Headed;
    fn enclosed(&mut self, enclosed: &Enclosed, layout: Layout) -> Enclosed;
    fn bare(&mut self, bare: &Bare) -> Bare;
    fn emit_parenthetical_payload(&mut self, payload: &str);
    fn delimiter(&self, boundary: Boundary) -> &'static DelimiterSpec;
    fn separator(&self, separator: Separator) -> &'static SeparatorSpec;
    fn emit(&mut self, text: &str);
}

impl Rendering for Printer {
    fn materialize(&mut self, portion: Portion) -> Portion {
        self.portion(&portion, Layout::Flat)
    }

    fn delineation(&mut self, delineation: &Delineation, layout: Layout) -> Delineation {
        let mut portions = Vec::with_capacity(delineation.portions.len());
        for portion in &delineation.portions {
            if !self.output.is_empty() {
                match layout {
                    Layout::Flat => self.emit(" "),
                }
            }
            portions.push(self.portion(portion, layout));
        }
        Delineation { portions }
    }

    fn portion(&mut self, portion: &Portion, layout: Layout) -> Portion {
        let start = self.output.len();
        match portion {
            Portion::Headed(_, headed) => {
                let anatomy = self.headed(headed, layout);
                Portion::Headed(
                    Extent {
                        start,
                        end: self.output.len(),
                    },
                    anatomy,
                )
            }
            Portion::Enclosed(_, enclosed) => {
                let anatomy = self.enclosed(enclosed, layout);
                Portion::Enclosed(
                    Extent {
                        start,
                        end: self.output.len(),
                    },
                    anatomy,
                )
            }
            Portion::Bare(_, bare) => {
                let anatomy = self.bare(bare);
                Portion::Bare(
                    Extent {
                        start,
                        end: self.output.len(),
                    },
                    anatomy,
                )
            }
        }
    }

    fn headed(&mut self, headed: &Headed, layout: Layout) -> Headed {
        self.emit(headed.head.as_ref());
        self.emit(self.separator(headed.separator).text);
        Headed {
            head: Symbol(headed.head.0.to_owned()),
            separator: headed.separator,
            body: Box::new(self.portion(&headed.body, layout)),
        }
    }

    fn enclosed(&mut self, enclosed: &Enclosed, layout: Layout) -> Enclosed {
        match enclosed {
            Enclosed::Structural(enclosed) => {
                let boundary = Boundary::Structural(enclosed.enclosure);
                let delimiter = self.delimiter(boundary);
                self.emit(delimiter.opening);
                let mut printed = Vec::with_capacity(enclosed.portions.len());
                for portion in &enclosed.portions {
                    if !printed.is_empty() {
                        match layout {
                            Layout::Flat => self.emit(" "),
                        }
                    }
                    printed.push(self.portion(portion, layout));
                }
                self.emit(delimiter.closing);
                Enclosed::Structural(StructuralEnclosed {
                    enclosure: enclosed.enclosure,
                    portions: printed,
                })
            }
            Enclosed::Opaque(enclosed) => {
                let boundary = Boundary::Opaque(enclosed.boundary);
                let delimiter = self.delimiter(boundary);
                self.emit(delimiter.opening);
                match enclosed.boundary {
                    OpaqueBoundary::Dialect(DialectBoundary::Parentheses) => {
                        self.emit_parenthetical_payload(&enclosed.content)
                    }
                    OpaqueBoundary::CurlyQuote => self.emit(&enclosed.content),
                }
                self.emit(delimiter.closing);
                Enclosed::Opaque(OpaqueEnclosed {
                    boundary: enclosed.boundary,
                    content: enclosed.content.to_owned(),
                })
            }
        }
    }

    fn bare(&mut self, bare: &Bare) -> Bare {
        self.emit(bare.symbol.as_ref());
        Bare {
            symbol: Symbol(bare.symbol.0.to_owned()),
        }
    }

    fn emit_parenthetical_payload(&mut self, payload: &str) {
        let delimiter = self.delimiter(Boundary::Opaque(OpaqueBoundary::Dialect(
            DialectBoundary::Parentheses,
        )));
        let DelimiterHandling::BalancedOpaque {
            escape: Some(escape),
        } = delimiter.handling
        else {
            unreachable!("parentheses have an escape specification")
        };
        let mut unmatched_openings = Vec::new();
        for character in payload.chars() {
            let character = character.to_string();
            if character == escape {
                self.emit(escape);
                self.emit(escape);
            } else if character == delimiter.opening {
                unmatched_openings.push(self.output.len());
                self.emit(delimiter.opening);
            } else if character == delimiter.closing {
                if unmatched_openings.pop().is_some() {
                    self.emit(delimiter.closing);
                } else {
                    self.emit(escape);
                    self.emit(delimiter.closing);
                }
            } else {
                self.emit(&character);
            }
        }
        for position in unmatched_openings.into_iter().rev() {
            self.output.insert_str(position, escape);
        }
    }

    fn delimiter(&self, boundary: Boundary) -> &'static DelimiterSpec {
        DELIMITERS
            .iter()
            .find(|delimiter| delimiter.boundary == boundary)
            .expect("every parsed boundary has one universal delimiter specification")
    }

    fn separator(&self, separator: Separator) -> &'static SeparatorSpec {
        SEPARATORS
            .iter()
            .find(|specification| specification.separator == separator)
            .expect("every Separator has one universal separator specification")
    }

    fn emit(&mut self, text: &str) {
        self.output.push_str(text);
    }
}

struct DelimiterSpec {
    boundary: Boundary,
    opening: &'static str,
    closing: &'static str,
    handling: DelimiterHandling,
}

struct SeparatorSpec {
    separator: Separator,
    text: &'static str,
}

enum DelimiterHandling {
    Structural,
    BalancedOpaque { escape: Option<&'static str> },
}

static DELIMITERS: [DelimiterSpec; 6] = [
    DelimiterSpec {
        boundary: Boundary::Structural(StructuralEnclosure::Braced),
        opening: "{",
        closing: "}",
        handling: DelimiterHandling::Structural,
    },
    DelimiterSpec {
        boundary: Boundary::Structural(StructuralEnclosure::Bracketed),
        opening: "[",
        closing: "]",
        handling: DelimiterHandling::Structural,
    },
    DelimiterSpec {
        boundary: Boundary::Structural(StructuralEnclosure::Guillemets),
        opening: "«",
        closing: "»",
        handling: DelimiterHandling::Structural,
    },
    DelimiterSpec {
        boundary: Boundary::Structural(StructuralEnclosure::Angled),
        opening: "<",
        closing: ">",
        handling: DelimiterHandling::Structural,
    },
    DelimiterSpec {
        boundary: Boundary::Opaque(OpaqueBoundary::CurlyQuote),
        opening: "“",
        closing: "”",
        handling: DelimiterHandling::BalancedOpaque { escape: None },
    },
    DelimiterSpec {
        boundary: Boundary::Opaque(OpaqueBoundary::Dialect(DialectBoundary::Parentheses)),
        opening: "(",
        closing: ")",
        handling: DelimiterHandling::BalancedOpaque { escape: Some("\\") },
    },
];

static SEPARATORS: [SeparatorSpec; 3] = [
    SeparatorSpec {
        separator: Separator::Period,
        text: ".",
    },
    SeparatorSpec {
        separator: Separator::Exclamation,
        text: "!",
    },
    SeparatorSpec {
        separator: Separator::Colon,
        text: ":",
    },
];

struct TextHasher;

trait Hashing {
    fn hash(&self, text: &str) -> ContentHash;
}

impl Hashing for TextHasher {
    fn hash(&self, text: &str) -> ContentHash {
        let mut value = 0xcbf29ce484222325_u64;
        for byte in text.bytes() {
            value ^= u64::from(byte);
            value = value.wrapping_mul(0x100000001b3);
        }
        ContentHash(value)
    }
}

struct Parser<'input> {
    input: &'input str,
    cursor: usize,
}

trait Parsing {
    fn delineate_document(&mut self) -> Result<Delineation, Fault>;
    fn portions_until(
        &mut self,
        closing: Option<(&'static str, usize)>,
    ) -> Result<Vec<Portion>, Fault>;
    fn portion(&mut self) -> Result<Portion, Fault>;
    fn enclosed(&mut self, start: usize, delimiter: &DelimiterSpec) -> Result<Portion, Fault>;
    fn balanced_opaque(
        &mut self,
        start: usize,
        delimiter: &DelimiterSpec,
        escape: Option<&'static str>,
    ) -> Result<Portion, Fault>;
    fn bare_or_headed(&mut self) -> Result<Portion, Fault>;
    fn skip_trivia(&mut self);
    fn matching_opening(&self) -> Option<&'static DelimiterSpec>;
    fn matching_closer(&self) -> Option<&'static DelimiterSpec>;
    fn next_character(&self) -> Option<char>;
    fn advance_character(&mut self);
    fn separator_from(&self, character: char) -> Option<Separator>;
    fn fault(&self, start: usize, problem: FaultProblem) -> Fault;
}

impl Parsing for Parser<'_> {
    fn delineate_document(&mut self) -> Result<Delineation, Fault> {
        self.portions_until(None)
            .map(|portions| Delineation { portions })
    }

    fn portions_until(
        &mut self,
        closing: Option<(&'static str, usize)>,
    ) -> Result<Vec<Portion>, Fault> {
        let mut portions = Vec::new();
        loop {
            self.skip_trivia();
            if let Some((expected, _)) = closing {
                if self.input[self.cursor..].starts_with(expected) {
                    self.cursor += expected.len();
                    return Ok(portions);
                }
            }
            if self.cursor == self.input.len() {
                return match closing {
                    Some((_, start)) => Err(self.fault(start, FaultProblem::UnclosedDelimiter)),
                    None => Ok(portions),
                };
            }
            if let Some(closer) = self.matching_closer() {
                return Err(Fault {
                    extent: Extent {
                        start: self.cursor,
                        end: self.cursor + closer.closing.len(),
                    },
                    problem: FaultProblem::UnexpectedCloser,
                });
            }
            portions.push(self.portion()?);
        }
    }

    fn portion(&mut self) -> Result<Portion, Fault> {
        let start = self.cursor;
        match self.matching_opening() {
            Some(delimiter) => {
                self.cursor += delimiter.opening.len();
                match delimiter.handling {
                    DelimiterHandling::Structural => self.enclosed(start, delimiter),
                    DelimiterHandling::BalancedOpaque { escape } => {
                        self.balanced_opaque(start, delimiter, escape)
                    }
                }
            }
            None => self.bare_or_headed(),
        }
    }

    fn enclosed(&mut self, start: usize, delimiter: &DelimiterSpec) -> Result<Portion, Fault> {
        let portions = self.portions_until(Some((delimiter.closing, start)))?;
        Ok(Portion::Enclosed(
            Extent {
                start,
                end: self.cursor,
            },
            match delimiter.boundary {
                Boundary::Structural(enclosure) => Enclosed::Structural(StructuralEnclosed {
                    enclosure,
                    portions,
                }),
                Boundary::Opaque(_) => {
                    unreachable!("a structural delimiter has a structural boundary")
                }
            },
        ))
    }

    fn balanced_opaque(
        &mut self,
        start: usize,
        delimiter: &DelimiterSpec,
        escape: Option<&'static str>,
    ) -> Result<Portion, Fault> {
        let mut content = String::new();
        let mut depth = 1_usize;
        while self.cursor < self.input.len() {
            if escape.is_some_and(|escape| self.input[self.cursor..].starts_with(escape)) {
                self.cursor += escape.expect("checked above").len();
                if let Some(character) = self.next_character() {
                    content.push(character);
                    self.advance_character();
                } else {
                    return Err(self.fault(start, FaultProblem::UnclosedDelimiter));
                }
            } else if self.input[self.cursor..].starts_with(delimiter.opening) {
                depth += 1;
                content.push_str(delimiter.opening);
                self.cursor += delimiter.opening.len();
            } else if self.input[self.cursor..].starts_with(delimiter.closing) {
                depth -= 1;
                if depth == 0 {
                    self.cursor += delimiter.closing.len();
                    return Ok(Portion::Enclosed(
                        Extent {
                            start,
                            end: self.cursor,
                        },
                        match delimiter.boundary {
                            Boundary::Opaque(boundary) => {
                                Enclosed::Opaque(OpaqueEnclosed { boundary, content })
                            }
                            Boundary::Structural(_) => {
                                unreachable!("an opaque delimiter has an opaque boundary")
                            }
                        },
                    ));
                }
                content.push_str(delimiter.closing);
                self.cursor += delimiter.closing.len();
            } else {
                content.push(self.next_character().expect("cursor is in bounds"));
                self.advance_character();
            }
        }
        Err(self.fault(start, FaultProblem::UnclosedDelimiter))
    }

    fn bare_or_headed(&mut self) -> Result<Portion, Fault> {
        let start = self.cursor;
        let mut separator = None;
        while let Some(character) = self.next_character() {
            if character.is_whitespace()
                || self.matching_opening().is_some()
                || self.matching_closer().is_some()
            {
                break;
            }
            if let Some(found) = self.separator_from(character) {
                separator = Some(found);
                break;
            }
            self.advance_character();
        }
        if self.cursor == start {
            let end = self
                .next_character()
                .map_or(start, |character| start + character.len_utf8());
            return Err(Fault {
                extent: Extent { start, end },
                problem: FaultProblem::MissingHead,
            });
        }
        let symbol = Symbol(self.input[start..self.cursor].to_owned());
        match separator {
            Some(separator) => {
                self.advance_character();
                if self.cursor == self.input.len()
                    || self.next_character().is_some_and(char::is_whitespace)
                    || self.matching_closer().is_some()
                {
                    return Err(self.fault(self.cursor, FaultProblem::MissingBody));
                }
                let body = self.portion()?;
                let end = body.as_ref().end;
                Ok(Portion::Headed(
                    Extent { start, end },
                    Headed {
                        head: symbol,
                        separator,
                        body: Box::new(body),
                    },
                ))
            }
            None => Ok(Portion::Bare(
                Extent {
                    start,
                    end: self.cursor,
                },
                Bare { symbol },
            )),
        }
    }

    fn skip_trivia(&mut self) {
        loop {
            while self.next_character().is_some_and(char::is_whitespace) {
                self.advance_character();
            }
            if !self.input[self.cursor..].starts_with(";;") {
                return;
            }
            while let Some(character) = self.next_character() {
                self.advance_character();
                if character == '\n' {
                    break;
                }
            }
        }
    }

    fn matching_opening(&self) -> Option<&'static DelimiterSpec> {
        DELIMITERS
            .iter()
            .find(|delimiter| self.input[self.cursor..].starts_with(delimiter.opening))
    }

    fn matching_closer(&self) -> Option<&'static DelimiterSpec> {
        DELIMITERS
            .iter()
            .find(|delimiter| self.input[self.cursor..].starts_with(delimiter.closing))
    }

    fn next_character(&self) -> Option<char> {
        self.input[self.cursor..].chars().next()
    }

    fn advance_character(&mut self) {
        if let Some(character) = self.next_character() {
            self.cursor += character.len_utf8();
        }
    }

    fn separator_from(&self, character: char) -> Option<Separator> {
        SEPARATORS
            .iter()
            .find(|specification| specification.text.starts_with(character))
            .map(|specification| specification.separator)
    }

    fn fault(&self, start: usize, problem: FaultProblem) -> Fault {
        Fault {
            extent: Extent {
                start,
                end: self.cursor,
            },
            problem,
        }
    }
}

impl Copy for Separator {}

impl Clone for Separator {
    fn clone(&self) -> Self {
        *self
    }
}

impl Clone for Symbol {
    fn clone(&self) -> Self {
        Self(self.0.to_owned())
    }
}

impl Clone for Extent {
    fn clone(&self) -> Self {
        Self {
            start: self.start,
            end: self.end,
        }
    }
}

impl Clone for Bare {
    fn clone(&self) -> Self {
        Self {
            symbol: self.symbol.clone(),
        }
    }
}

impl Clone for Headed {
    fn clone(&self) -> Self {
        Self {
            head: self.head.clone(),
            separator: self.separator,
            body: Box::new((*self.body).clone()),
        }
    }
}

impl Clone for StructuralEnclosed {
    fn clone(&self) -> Self {
        Self {
            enclosure: self.enclosure,
            portions: self.portions.iter().map(Clone::clone).collect(),
        }
    }
}

impl Clone for OpaqueEnclosed {
    fn clone(&self) -> Self {
        Self {
            boundary: self.boundary,
            content: self.content.to_owned(),
        }
    }
}

impl Clone for Enclosed {
    fn clone(&self) -> Self {
        match self {
            Self::Structural(enclosed) => Self::Structural(enclosed.clone()),
            Self::Opaque(enclosed) => Self::Opaque(enclosed.clone()),
        }
    }
}

impl Clone for Portion {
    fn clone(&self) -> Self {
        match self {
            Self::Headed(extent, headed) => Self::Headed(extent.clone(), headed.clone()),
            Self::Enclosed(extent, enclosed) => Self::Enclosed(extent.clone(), enclosed.clone()),
            Self::Bare(extent, bare) => Self::Bare(extent.clone(), bare.clone()),
        }
    }
}

impl Clone for Delineation {
    fn clone(&self) -> Self {
        Self {
            portions: self.portions.iter().map(Clone::clone).collect(),
        }
    }
}

impl Copy for BareExpectation {}

impl Clone for BareExpectation {
    fn clone(&self) -> Self {
        *self
    }
}

impl Copy for Enclosure {}

impl Clone for Enclosure {
    fn clone(&self) -> Self {
        *self
    }
}

impl Copy for StructuralEnclosure {}

impl Clone for StructuralEnclosure {
    fn clone(&self) -> Self {
        *self
    }
}

impl Copy for OpaqueBoundary {}

impl Clone for OpaqueBoundary {
    fn clone(&self) -> Self {
        *self
    }
}

impl Copy for Boundary {}

impl Clone for Boundary {
    fn clone(&self) -> Self {
        *self
    }
}

impl Copy for Layout {}

impl Clone for Layout {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> PartialEq for Text<T> {
    fn eq(&self, other: &Self) -> bool {
        self.normalized == other.normalized
    }
}

impl<T> Eq for Text<T> {}

impl PartialEq for ContentHash {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for ContentHash {}

impl PartialEq for Symbol {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for Symbol {}

impl PartialEq for Extent {
    fn eq(&self, other: &Self) -> bool {
        self.start == other.start && self.end == other.end
    }
}

impl Eq for Extent {}

impl PartialEq for Separator {
    fn eq(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (Self::Period, Self::Period)
                | (Self::Exclamation, Self::Exclamation)
                | (Self::Colon, Self::Colon)
        )
    }
}

impl Eq for Separator {}

impl PartialEq for BareExpectation {
    fn eq(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (Self::Symbol, Self::Symbol) | (Self::String, Self::String)
        )
    }
}

impl Eq for BareExpectation {}

impl PartialEq for Enclosure {
    fn eq(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (Self::Braced, Self::Braced)
                | (Self::Bracketed, Self::Bracketed)
                | (Self::Guillemets, Self::Guillemets)
                | (Self::Angled, Self::Angled)
                | (Self::CurlyQuote, Self::CurlyQuote)
        )
    }
}

impl Eq for Enclosure {}

impl PartialEq for StructuralEnclosure {
    fn eq(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (Self::Braced, Self::Braced)
                | (Self::Bracketed, Self::Bracketed)
                | (Self::Guillemets, Self::Guillemets)
                | (Self::Angled, Self::Angled)
        )
    }
}

impl Eq for StructuralEnclosure {}

impl PartialEq for OpaqueBoundary {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::CurlyQuote, Self::CurlyQuote) => true,
            (Self::Dialect(left), Self::Dialect(right)) => left == right,
            _ => false,
        }
    }
}

impl Eq for OpaqueBoundary {}

impl PartialEq for Boundary {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Structural(left), Self::Structural(right)) => left == right,
            (Self::Opaque(left), Self::Opaque(right)) => left == right,
            _ => false,
        }
    }
}

impl Eq for Boundary {}

impl Copy for DialectBoundary {}

impl Clone for DialectBoundary {
    fn clone(&self) -> Self {
        *self
    }
}

impl PartialEq for DialectBoundary {
    fn eq(&self, other: &Self) -> bool {
        matches!((self, other), (Self::Parentheses, Self::Parentheses))
    }
}

impl Eq for DialectBoundary {}

impl PartialEq for Portion {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Headed(left_extent, left), Self::Headed(right_extent, right)) => {
                left_extent == right_extent && left == right
            }
            (Self::Enclosed(left_extent, left), Self::Enclosed(right_extent, right)) => {
                left_extent == right_extent && left == right
            }
            (Self::Bare(left_extent, left), Self::Bare(right_extent, right)) => {
                left_extent == right_extent && left == right
            }
            _ => false,
        }
    }
}

impl Eq for Portion {}

impl PartialEq for Headed {
    fn eq(&self, other: &Self) -> bool {
        self.head == other.head && self.separator == other.separator && self.body == other.body
    }
}

impl Eq for Headed {}

impl PartialEq for Enclosed {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Structural(left), Self::Structural(right)) => left == right,
            (Self::Opaque(left), Self::Opaque(right)) => left == right,
            _ => false,
        }
    }
}

impl Eq for Enclosed {}

impl PartialEq for StructuralEnclosed {
    fn eq(&self, other: &Self) -> bool {
        self.enclosure == other.enclosure && self.portions == other.portions
    }
}

impl Eq for StructuralEnclosed {}

impl PartialEq for OpaqueEnclosed {
    fn eq(&self, other: &Self) -> bool {
        self.boundary == other.boundary && self.content == other.content
    }
}

impl Eq for OpaqueEnclosed {}

impl PartialEq for Bare {
    fn eq(&self, other: &Self) -> bool {
        self.symbol == other.symbol
    }
}

impl Eq for Bare {}

impl PartialEq for Delineation {
    fn eq(&self, other: &Self) -> bool {
        self.portions == other.portions
    }
}

impl Eq for Delineation {}

impl PartialEq for Fault {
    fn eq(&self, other: &Self) -> bool {
        self.extent == other.extent && self.problem == other.problem
    }
}

impl Eq for Fault {}

impl PartialEq for FaultProblem {
    fn eq(&self, other: &Self) -> bool {
        matches!(
            (self, other),
            (Self::UnexpectedCloser, Self::UnexpectedCloser)
                | (Self::UnclosedDelimiter, Self::UnclosedDelimiter)
                | (Self::MissingHead, Self::MissingHead)
                | (Self::MissingBody, Self::MissingBody)
                | (Self::ExpectedOnePortion, Self::ExpectedOnePortion)
                | (Self::ExpectedShape, Self::ExpectedShape)
                | (Self::ExpectedBareSymbol, Self::ExpectedBareSymbol)
                | (Self::InvalidSignedInteger, Self::InvalidSignedInteger)
                | (Self::IntegerOutOfRange, Self::IntegerOutOfRange)
                | (Self::InvalidDecimal, Self::InvalidDecimal)
                | (Self::NonFiniteDecimal, Self::NonFiniteDecimal)
        )
    }
}

impl Eq for FaultProblem {}

macro_rules! debug_as_display {
    ($($type:ty),+ $(,)?) => {
        $(impl fmt::Debug for $type {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(formatter, "{}", stringify!($type))
            }
        })+
    };
}

debug_as_display!(
    ContentHash,
    Symbol,
    Extent,
    Separator,
    BareExpectation,
    Enclosure,
    StructuralEnclosure,
    OpaqueBoundary,
    Boundary,
    DialectBoundary,
    Portion,
    Headed,
    Enclosed,
    StructuralEnclosed,
    OpaqueEnclosed,
    Bare,
    Delineation,
    Fault,
    FaultProblem,
    Layout,
);

impl<T> fmt::Debug for Text<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Text")
    }
}
