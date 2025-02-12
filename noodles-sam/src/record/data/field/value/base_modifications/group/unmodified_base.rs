use std::{error, fmt};

/// The unmodified base as reported by the sequencer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnmodifiedBase {
    /// Adenine.
    A,
    /// Cytosine.
    C,
    /// Guanine.
    G,
    /// Thymine.
    T,
    /// Uracil.
    U,
    /// Any base.
    N,
}

/// An error returned when a base modifications group unmodified base fails to parse.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParseError {
    /// The input is invalid.
    Invalid,
}

impl error::Error for ParseError {}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid => write!(f, "invalid input"),
        }
    }
}

impl TryFrom<u8> for UnmodifiedBase {
    type Error = ParseError;

    fn try_from(b: u8) -> Result<Self, Self::Error> {
        match b {
            b'A' => Ok(Self::A),
            b'C' => Ok(Self::C),
            b'G' => Ok(Self::G),
            b'T' => Ok(Self::T),
            b'U' => Ok(Self::U),
            b'N' => Ok(Self::N),
            _ => Err(ParseError::Invalid),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_try_from_u8_for_unmodified_base() {
        fn t(b: u8, expected: UnmodifiedBase) {
            assert_eq!(UnmodifiedBase::try_from(b), Ok(expected));
        }

        t(b'A', UnmodifiedBase::A);
        t(b'C', UnmodifiedBase::C);
        t(b'G', UnmodifiedBase::G);
        t(b'T', UnmodifiedBase::T);
        t(b'U', UnmodifiedBase::U);
        t(b'N', UnmodifiedBase::N);

        assert_eq!(UnmodifiedBase::try_from(b'n'), Err(ParseError::Invalid));
    }
}
