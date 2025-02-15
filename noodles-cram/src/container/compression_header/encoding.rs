pub mod codec;
mod kind;

pub use self::kind::Kind;

use std::io::{self, Write};

use crate::io::{
    reader::container::slice::records::ExternalDataReaders,
    writer::container::slice::records::ExternalDataWriters, BitReader, BitWriter,
};

pub trait Decode<'de> {
    type Value;

    fn decode(
        &self,
        core_data_reader: &mut BitReader<'de>,
        external_data_readers: &mut ExternalDataReaders<'de>,
    ) -> io::Result<Self::Value>;
}

pub trait Encode<'en> {
    type Value;

    fn encode<X>(
        &self,
        core_data_writer: &mut BitWriter,
        external_data_writers: &mut ExternalDataWriters<X>,
        value: Self::Value,
    ) -> io::Result<()>
    where
        X: Write;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Encoding<C>(C);

impl<C> Encoding<C> {
    pub fn new(codec: C) -> Self {
        Self(codec)
    }

    pub fn get(&self) -> &C {
        &self.0
    }
}

impl<'de, C> Encoding<C>
where
    C: Decode<'de>,
{
    pub fn decode(
        &self,
        core_data_reader: &mut BitReader<'de>,
        external_data_readers: &mut ExternalDataReaders<'de>,
    ) -> io::Result<C::Value> {
        self.get().decode(core_data_reader, external_data_readers)
    }
}

impl<'en, C> Encoding<C>
where
    C: Encode<'en>,
{
    pub fn encode<X>(
        &self,
        core_data_writer: &mut BitWriter,
        external_data_writers: &mut ExternalDataWriters<X>,
        value: C::Value,
    ) -> io::Result<()>
    where
        X: Write,
    {
        self.get()
            .encode(core_data_writer, external_data_writers, value)
    }
}
