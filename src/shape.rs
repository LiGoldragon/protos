/// The fixed structural vocabulary shared by every Protos dialect.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Shape {
    Bare,
    CurlyQuoted,
    DottedCurlyQuoted,
    Parenthesized,
    SquareBracketed,
    Braced,
    DottedParenthesized,
    DottedSquareBracketed,
    DottedBraced,
}

/// A type whose textual shape selects one of its own possibilities.
///
/// This is discrimination only. The selected type owns its own realisation;
/// no context interior belongs here.
pub trait ShapeDefined: Sized {
    type Selection;

    fn shapes() -> &'static [Shape];
    fn select(shape: Shape, head: Option<&crate::Head>) -> Option<Self::Selection>;
}
