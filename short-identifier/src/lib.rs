//! The family-wide short base36 identifier and its one canonical mint.
//!
//! A [`ShortCode`] is an agent-addressable, four-to-seven-character base36
//! value. [`ShortIdentifierMint`] is the sole allocation algorithm: it draws a
//! bounded number of random candidates, then deterministically scans the
//! remaining range before growing the code length. Consumers own their live
//! collision set but do not reimplement allocation.

use std::collections::BTreeSet;

use thiserror::Error;

/// The shortest code issued by the family mint.
pub const MINIMUM_CODE_LENGTH: usize = 4;
/// The longest code issued by the family mint.
pub const MAXIMUM_CODE_LENGTH: usize = 7;
const CODE_RADIX: u64 = 36;
const RANDOM_DRAWS_PER_LENGTH: usize = 128;

/// A validated lowercase base36 code issued by [`ShortIdentifierMint`].
#[derive(
    Clone, Debug, Eq, Ord, PartialEq, PartialOrd, rkyv::Archive, rkyv::Deserialize, rkyv::Serialize,
)]
pub struct ShortCode(String);

impl ShortCode {
    /// Validate a code received at a typed boundary.
    pub fn new(code: String) -> Result<Self, ShortIdentifierError> {
        let length = code.chars().count();
        if !(MINIMUM_CODE_LENGTH..=MAXIMUM_CODE_LENGTH).contains(&length) {
            return Err(ShortIdentifierError::InvalidLength { found: length });
        }
        if !code.bytes().all(is_base36_byte) {
            return Err(ShortIdentifierError::InvalidAlphabet);
        }
        Ok(Self(code))
    }

    /// The canonical lowercase base36 text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for ShortCode {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

/// A value that exposes its standard short identifier.
///
/// The trait keeps agent addressability structural: a caller can obtain the
/// code without knowing the value's storage or transport representation.
pub trait ShortIdentifier {
    /// The short code assigned to this value.
    fn short_identifier(&self) -> &ShortCode;
}

impl ShortIdentifier for ShortCode {
    fn short_identifier(&self) -> &ShortCode {
        self
    }
}

/// A typed source of random draws used by [`ShortIdentifierMint`].
///
/// The production source uses the operating-system CSPRNG; tests can inject a
/// deterministic source without changing the minting algorithm.
pub trait RandomDraw {
    /// Produce one uniformly unconstrained 64-bit draw.
    fn draw_u64(&mut self) -> Result<u64, ShortIdentifierError>;
}

/// The operating-system random source used by [`ShortIdentifierMint::mint`].
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemRandomDraw;

impl RandomDraw for SystemRandomDraw {
    fn draw_u64(&mut self) -> Result<u64, ShortIdentifierError> {
        let mut bytes = [0_u8; std::mem::size_of::<u64>()];
        getrandom::fill(&mut bytes).map_err(|_| ShortIdentifierError::RandomnessUnavailable)?;
        Ok(u64::from_be_bytes(bytes))
    }
}

/// The canonical mint over the set of short codes already in use.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShortIdentifierMint {
    used: BTreeSet<ShortCode>,
    minimum_length: usize,
    maximum_length: usize,
}

impl ShortIdentifierMint {
    /// Build the production four-to-seven-character mint over the live code
    /// set. Equal codes deduplicate before any allocation attempt.
    pub fn new(codes: impl IntoIterator<Item = ShortCode>) -> Self {
        Self {
            used: codes.into_iter().collect(),
            minimum_length: MINIMUM_CODE_LENGTH,
            maximum_length: MAXIMUM_CODE_LENGTH,
        }
    }

    /// Mint a code from the operating-system random source.
    pub fn mint(&self) -> Result<ShortCode, ShortIdentifierError> {
        self.mint_with(&mut SystemRandomDraw)
    }

    /// Mint with an injected draw source. This is public so a daemon can own
    /// the entropy policy while tests retain deterministic witnesses.
    pub fn mint_with(&self, draw: &mut impl RandomDraw) -> Result<ShortCode, ShortIdentifierError> {
        for length in self.minimum_length..=self.maximum_length {
            match self.mint_at_length(length, draw) {
                Ok(code) => return Ok(code),
                Err(ShortIdentifierError::Saturated { .. }) => continue,
                Err(error) => return Err(error),
            }
        }
        Err(ShortIdentifierError::Exhausted {
            minimum: self.minimum_length,
            maximum: self.maximum_length,
        })
    }

    fn mint_at_length(
        &self,
        length: usize,
        draw: &mut impl RandomDraw,
    ) -> Result<ShortCode, ShortIdentifierError> {
        let range = CodeRange::new(length, self.minimum_length);
        for _ in 0..RANDOM_DRAWS_PER_LENGTH {
            let candidate =
                range.code_from_value(range.first_value + draw.draw_u64()? % range.value_count);
            if !self.used.contains(&candidate) {
                return Ok(candidate);
            }
        }
        range
            .first_available(&self.used)
            .ok_or(ShortIdentifierError::Saturated { length })
    }

    #[cfg(test)]
    fn with_test_bounds(
        codes: impl IntoIterator<Item = ShortCode>,
        minimum: usize,
        maximum: usize,
    ) -> Self {
        Self {
            used: codes.into_iter().collect(),
            minimum_length: minimum,
            maximum_length: maximum,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CodeRange {
    first_value: u64,
    value_count: u64,
    pad_to: usize,
}

impl CodeRange {
    fn new(length: usize, minimum_length: usize) -> Self {
        let first_value = if length == minimum_length {
            0
        } else {
            radix_power(length - 1)
        };
        Self {
            first_value,
            value_count: radix_power(length) - first_value,
            pad_to: minimum_length,
        }
    }

    fn code_from_value(self, mut value: u64) -> ShortCode {
        let mut digits = Vec::new();
        while value > 0 {
            digits.push(base36_character((value % CODE_RADIX) as u8));
            value /= CODE_RADIX;
        }
        while digits.len() < self.pad_to {
            digits.push('0');
        }
        ShortCode(digits.iter().rev().collect())
    }

    fn first_available(self, used: &BTreeSet<ShortCode>) -> Option<ShortCode> {
        let end = self.first_value + self.value_count;
        (self.first_value..end)
            .map(|value| self.code_from_value(value))
            .find(|code| !used.contains(code))
    }
}

/// A structural short-identifier failure.
#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum ShortIdentifierError {
    /// A received code does not have the family-wide four-to-seven length.
    #[error("short identifier length {found} is outside the four-to-seven-character range")]
    InvalidLength { found: usize },
    /// A received code contains a character outside lowercase base36.
    #[error("short identifier contains a non-base36 character")]
    InvalidAlphabet,
    /// The operating-system CSPRNG could not provide a draw.
    #[error("short identifier randomness is unavailable")]
    RandomnessUnavailable,
    /// Every code at one length is already occupied.
    #[error("short identifier space at {length} characters is saturated")]
    Saturated { length: usize },
    /// Every code between the configured bounds is already occupied.
    #[error("short identifier space from {minimum} to {maximum} characters is exhausted")]
    Exhausted { minimum: usize, maximum: usize },
}

fn is_base36_byte(byte: u8) -> bool {
    byte.is_ascii_digit() || byte.is_ascii_lowercase()
}

fn base36_character(digit: u8) -> char {
    match digit {
        0..=9 => char::from(b'0' + digit),
        10..=35 => char::from(b'a' + digit - 10),
        _ => unreachable!("the base36 remainder is constrained by the radix"),
    }
}

fn radix_power(exponent: usize) -> u64 {
    (0..exponent).fold(1, |value, _| value * CODE_RADIX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug)]
    struct SeededDraw {
        values: Vec<u64>,
        cursor: usize,
    }

    impl SeededDraw {
        fn new(values: Vec<u64>) -> Self {
            Self { values, cursor: 0 }
        }
    }

    impl RandomDraw for SeededDraw {
        fn draw_u64(&mut self) -> Result<u64, ShortIdentifierError> {
            let value = self.values[self.cursor % self.values.len()];
            self.cursor += 1;
            Ok(value)
        }
    }

    fn codes_at_length(length: usize) -> Vec<ShortCode> {
        let range = CodeRange::new(length, length);
        (range.first_value..range.first_value + range.value_count)
            .map(|value| range.code_from_value(value))
            .collect()
    }

    #[test]
    fn seeded_draws_mint_deterministically() {
        let mint = ShortIdentifierMint::new([]);
        let mut first_draw = SeededDraw::new(vec![42]);
        let mut second_draw = SeededDraw::new(vec![42]);
        assert_eq!(
            mint.mint_with(&mut first_draw).expect("first mint"),
            mint.mint_with(&mut second_draw).expect("second mint")
        );
    }

    #[test]
    fn collisions_fall_back_to_the_first_free_code() {
        let mut occupied = codes_at_length(1);
        let expected = occupied.pop().expect("one free code");
        let mint = ShortIdentifierMint::with_test_bounds(occupied, 1, 1);
        let mut draws = SeededDraw::new(vec![0]);
        assert_eq!(mint.mint_with(&mut draws).expect("fallback mint"), expected);
    }

    #[test]
    fn saturated_length_is_a_typed_outcome() {
        let mint = ShortIdentifierMint::with_test_bounds(codes_at_length(1), 1, 1);
        let mut draws = SeededDraw::new(vec![0]);
        assert_eq!(
            mint.mint_at_length(1, &mut draws),
            Err(ShortIdentifierError::Saturated { length: 1 })
        );
    }

    #[test]
    fn a_saturated_length_grows_to_the_next_length() {
        let mint = ShortIdentifierMint::with_test_bounds(codes_at_length(1), 1, 2);
        let mut draws = SeededDraw::new(vec![0]);
        assert_eq!(
            mint.mint_with(&mut draws)
                .expect("grown mint")
                .as_str()
                .len(),
            2
        );
    }

    #[test]
    fn full_bounds_return_a_typed_exhaustion() {
        let mint = ShortIdentifierMint::with_test_bounds(codes_at_length(1), 1, 1);
        let mut draws = SeededDraw::new(vec![0]);
        assert_eq!(
            mint.mint_with(&mut draws),
            Err(ShortIdentifierError::Exhausted {
                minimum: 1,
                maximum: 1,
            })
        );
    }
}
