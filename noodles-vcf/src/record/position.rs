//! VCF record position.

use std::{fmt, num, str::FromStr};

/// A VCF record position.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Position(usize);

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// An error returned when a raw VCF record position fails to parse.
pub type ParseError = num::ParseIntError;

impl FromStr for Position {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse().map(Self)
    }
}

impl From<usize> for Position {
    fn from(n: usize) -> Self {
        Self(n)
    }
}

impl From<Position> for usize {
    fn from(position: Position) -> Self {
        position.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fmt() {
        assert_eq!(Position(0).to_string(), "0");
        assert_eq!(Position(8).to_string(), "8");
        assert_eq!(Position(13).to_string(), "13");
    }

    #[test]
    fn test_from_str() {
        use std::num::IntErrorKind;

        assert_eq!("0".parse(), Ok(Position(0)));
        assert_eq!("8".parse(), Ok(Position(8)));
        assert_eq!("13".parse(), Ok(Position(13)));

        assert!(matches!("".parse::<Position>(), Err(e) if e.kind() == &IntErrorKind::Empty));
        assert!(
            matches!("ndls".parse::<Position>(), Err(e) if e.kind() == &IntErrorKind::InvalidDigit)
        );
        assert!(
            matches!("-1".parse::<Position>(), Err(e) if e.kind() == &IntErrorKind::InvalidDigit)
        );
    }

    #[test]
    fn test_from_usize_for_position() {
        assert_eq!(Position::from(0), Position(0));
        assert_eq!(Position::from(8), Position(8));
        assert_eq!(Position::from(13), Position(13));
    }

    #[test]
    fn test_from_position_for_usize() {
        assert_eq!(usize::from(Position::from(0)), 0);
        assert_eq!(usize::from(Position::from(8)), 8);
        assert_eq!(usize::from(Position::from(13)), 13);
    }
}
