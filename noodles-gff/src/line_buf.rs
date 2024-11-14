//! GFF lines.

use std::{error, fmt, str::FromStr};

use super::{directive_buf, record_buf, DirectiveBuf, RecordBuf};

const COMMENT_PREFIX: char = '#';

/// A GFF line.
#[derive(Clone, Debug, PartialEq)]
pub enum LineBuf {
    /// A directive (`##`).
    Directive(DirectiveBuf),
    /// A comment (`#`),
    Comment(String),
    /// A record.
    Record(RecordBuf),
}

impl fmt::Display for LineBuf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Directive(directive) => write!(f, "{directive}"),
            Self::Comment(comment) => write!(f, "{COMMENT_PREFIX}{comment}"),
            Self::Record(record) => write!(f, "{record}"),
        }
    }
}

/// An error returns when a raw GFF line fails to parse.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParseError {
    /// The directive is invalid.
    InvalidDirective(directive_buf::ParseError),
    /// The record is invalid.
    InvalidRecord(record_buf::ParseError),
}

impl error::Error for ParseError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Self::InvalidDirective(e) => Some(e),
            Self::InvalidRecord(e) => Some(e),
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDirective(_) => f.write_str("invalid directive"),
            Self::InvalidRecord(_) => f.write_str("invalid record"),
        }
    }
}

impl FromStr for LineBuf {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.starts_with(directive_buf::PREFIX) {
            s.parse()
                .map(Self::Directive)
                .map_err(ParseError::InvalidDirective)
        } else if let Some(t) = s.strip_prefix(COMMENT_PREFIX) {
            Ok(Self::Comment(t.into()))
        } else {
            s.parse()
                .map(Self::Record)
                .map_err(ParseError::InvalidRecord)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fmt() {
        let line = LineBuf::Directive(DirectiveBuf::GffVersion(Default::default()));
        assert_eq!(line.to_string(), "##gff-version 3");

        let line = LineBuf::Comment(String::from("format: gff3"));
        assert_eq!(line.to_string(), "#format: gff3");

        let line = LineBuf::Record(RecordBuf::default());
        assert_eq!(line.to_string(), ".\t.\t.\t1\t1\t.\t.\t.\t.")
    }

    #[test]
    fn test_from_str() {
        assert_eq!(
            "##gff-version 3".parse(),
            Ok(LineBuf::Directive(DirectiveBuf::GffVersion(
                Default::default()
            )))
        );

        assert_eq!(
            "#format: gff3".parse(),
            Ok(LineBuf::Comment(String::from("format: gff3")))
        );

        assert!(matches!(
            "sq0\tNOODLES\tgene\t8\t13\t.\t+\t.\tgene_id=ndls0;gene_name=gene0".parse(),
            Ok(LineBuf::Record(_))
        ));
    }
}
