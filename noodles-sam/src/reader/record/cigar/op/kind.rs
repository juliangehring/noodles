use std::io;

use crate::record::cigar::op::Kind;

pub(super) fn parse_kind(src: &mut &[u8]) -> io::Result<Kind> {
    let (n, rest) = src
        .split_first()
        .ok_or_else(|| io::Error::from(io::ErrorKind::UnexpectedEof))?;

    *src = rest;

    match n {
        b'M' => Ok(Kind::Match),
        b'I' => Ok(Kind::Insertion),
        b'D' => Ok(Kind::Deletion),
        b'N' => Ok(Kind::Skip),
        b'S' => Ok(Kind::SoftClip),
        b'H' => Ok(Kind::HardClip),
        b'P' => Ok(Kind::Pad),
        b'=' => Ok(Kind::SequenceMatch),
        b'X' => Ok(Kind::SequenceMismatch),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid CIGAR op kind",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_kind() -> io::Result<()> {
        fn t(mut src: &[u8], expected: Kind) -> io::Result<()> {
            assert_eq!(parse_kind(&mut src)?, expected);
            Ok(())
        }

        t(b"M", Kind::Match)?;
        t(b"I", Kind::Insertion)?;
        t(b"D", Kind::Deletion)?;
        t(b"N", Kind::Skip)?;
        t(b"S", Kind::SoftClip)?;
        t(b"H", Kind::HardClip)?;
        t(b"P", Kind::Pad)?;
        t(b"=", Kind::SequenceMatch)?;
        t(b"X", Kind::SequenceMismatch)?;

        assert!(matches!(
            parse_kind(&mut &[][..]),
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof,
        ));

        assert!(matches!(
            parse_kind(&mut &b"!"[..]),
            Err(e) if e.kind() == io::ErrorKind::InvalidData,
        ));

        Ok(())
    }
}
