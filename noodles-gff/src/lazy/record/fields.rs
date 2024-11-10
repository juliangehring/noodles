mod bounds;

use std::io;

use noodles_core::Position;

use self::bounds::Bounds;
use super::Attributes;
use crate::record::Strand;

#[derive(Clone, Eq, PartialEq)]
pub(super) struct Fields<'l> {
    src: &'l str,
    bounds: Bounds,
}

impl<'l> Fields<'l> {
    pub(super) fn try_new(src: &'l str) -> io::Result<Self> {
        Bounds::index(src).map(|bounds| Self { src, bounds })
    }

    pub fn reference_sequence_name(&self) -> &str {
        &self.src[self.bounds.reference_sequence_name_range()]
    }

    pub fn source(&self) -> &str {
        &self.src[self.bounds.source_range()]
    }

    pub fn ty(&self) -> &str {
        &self.src[self.bounds.type_range()]
    }

    pub fn start(&self) -> io::Result<Position> {
        let src = &self.src[self.bounds.start_range()];
        parse_position(src)
    }

    pub fn end(&self) -> io::Result<Position> {
        let src = &self.src[self.bounds.end_range()];
        parse_position(src)
    }

    pub fn score(&self) -> &str {
        &self.src[self.bounds.score_range()]
    }

    pub fn strand(&self) -> io::Result<Strand> {
        let src = &self.src[self.bounds.strand_range()];
        parse_strand(src)
    }

    pub fn phase(&self) -> &str {
        &self.src[self.bounds.phase_range()]
    }

    pub fn attributes(&self) -> Attributes<'_> {
        const MISSING: &str = ".";

        match &self.src[self.bounds.attributes_range()] {
            MISSING => Attributes::new(""),
            buf => Attributes::new(buf),
        }
    }
}

fn parse_position(s: &str) -> io::Result<Position> {
    s.parse()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

fn parse_strand(s: &str) -> io::Result<Strand> {
    s.parse()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}
