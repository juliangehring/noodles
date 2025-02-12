//! Indxed BCF reader.

mod builder;

pub use self::builder::Builder;

use std::io::{self, Read, Seek};

use noodles_bgzf as bgzf;
use noodles_core::Region;
use noodles_csi as csi;
use noodles_vcf as vcf;

use super::{
    header::StringMaps,
    lazy,
    reader::{Query, Records},
    Reader,
};

/// An indexed BCF reader.
pub struct IndexedReader<R> {
    inner: Reader<R>,
    index: csi::Index,
}

impl<R> IndexedReader<R>
where
    R: Read,
{
    /// Returns a reference to the underlying reader.
    pub fn get_ref(&self) -> &R {
        self.inner.get_ref()
    }

    /// Returns a mutable reference to the underlying reader.
    pub fn get_mut(&mut self) -> &mut R {
        self.inner.get_mut()
    }

    /// Returns the underlying reader.
    pub fn into_inner(self) -> R {
        self.inner.into_inner()
    }

    /// Returns the string maps.
    ///
    /// This is only built after reading the header using [`Self::read_header`].
    pub fn string_maps(&self) -> &StringMaps {
        self.inner.string_maps()
    }

    /// Reads the VCF header.
    pub fn read_header(&mut self) -> io::Result<vcf::Header> {
        self.inner.read_header()
    }

    /// Reads a single record.
    pub fn read_record(
        &mut self,
        header: &vcf::Header,
        record: &mut vcf::Record,
    ) -> io::Result<usize> {
        self.inner.read_record(header, record)
    }

    /// Reads a single record without eagerly decoding (most of) its fields.
    pub fn read_lazy_record(&mut self, record: &mut lazy::Record) -> io::Result<usize> {
        self.inner.read_lazy_record(record)
    }

    /// Returns an iterator over records starting from the current stream position.
    pub fn records<'r, 'h>(&'r mut self, header: &'h vcf::Header) -> Records<'r, 'h, R> {
        self.inner.records(header)
    }

    /// Returns an iterator over lazy records starting from the current stream position.
    pub fn lazy_records(&mut self) -> impl Iterator<Item = io::Result<lazy::Record>> + '_ {
        self.inner.lazy_records()
    }
}

impl<R> IndexedReader<bgzf::Reader<R>>
where
    R: Read,
{
    /// Creates an indexed BCF reader.
    pub fn new(inner: R, index: csi::Index) -> Self {
        Self {
            inner: Reader::new(inner),
            index,
        }
    }

    /// Returns the current virtual position of the underlying BGZF reader.
    pub fn virtual_position(&self) -> bgzf::VirtualPosition {
        self.inner.virtual_position()
    }
}

impl<R> IndexedReader<bgzf::Reader<R>>
where
    R: Read + Seek,
{
    /// Returns an iterator over records that intersects the given region.
    pub fn query<'r, 'h>(
        &'r mut self,
        header: &'h vcf::Header,
        region: &Region,
    ) -> io::Result<Query<'r, 'h, R>> {
        self.inner.query(header, &self.index, region)
    }
}
