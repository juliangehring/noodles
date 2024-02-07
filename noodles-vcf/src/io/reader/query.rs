use std::io::{self, Read, Seek};

use noodles_bgzf as bgzf;
use noodles_core::region::Interval;
use noodles_csi::{self as csi, binning_index::index::reference_sequence::bin::Chunk};

use super::Reader;
use crate::{variant::RecordBuf, Header};

/// An iterator over records of a VCF reader that intersects a given region.
///
/// This is created by calling [`Reader::query`].
pub struct Query<'r, 'h, R>
where
    R: Read + Seek,
{
    reader: Reader<csi::io::Query<'r, R>>,
    reference_sequence_name: Vec<u8>,
    interval: Interval,
    header: &'h Header,
    record: RecordBuf,
}

impl<'r, 'h, R> Query<'r, 'h, R>
where
    R: Read + Seek,
{
    pub(super) fn new(
        reader: &'r mut bgzf::Reader<R>,
        chunks: Vec<Chunk>,
        reference_sequence_name: Vec<u8>,
        interval: Interval,
        header: &'h Header,
    ) -> Self {
        Self {
            reader: Reader::new(csi::io::Query::new(reader, chunks)),
            reference_sequence_name,
            interval,
            header,
            record: RecordBuf::default(),
        }
    }

    fn next_record(&mut self) -> io::Result<Option<RecordBuf>> {
        self.reader
            .read_record(self.header, &mut self.record)
            .map(|n| match n {
                0 => None,
                _ => Some(self.record.clone()),
            })
    }
}

impl<'r, 'h, R> Iterator for Query<'r, 'h, R>
where
    R: Read + Seek,
{
    type Item = io::Result<RecordBuf>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.next_record() {
                Ok(Some(record)) => {
                    match intersects(&record, &self.reference_sequence_name, self.interval) {
                        Ok(true) => return Some(Ok(record)),
                        Ok(false) => {}
                        Err(e) => return Some(Err(e)),
                    }
                }
                Ok(None) => return None,
                Err(e) => return Some(Err(e)),
            }
        }
    }
}

pub(crate) fn intersects(
    record: &RecordBuf,
    reference_sequence_name: &[u8],
    region_interval: Interval,
) -> io::Result<bool> {
    use noodles_core::Position;

    let name = record.chromosome().to_string();

    let start = Position::try_from(usize::from(record.position()))
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    let end = record
        .end()
        .map(usize::from)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
        .and_then(|n| {
            Position::try_from(n).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
        })?;

    let record_interval = Interval::from(start..=end);

    Ok(name.as_bytes() == reference_sequence_name && record_interval.intersects(region_interval))
}
