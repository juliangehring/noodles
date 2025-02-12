use std::io::{self, Read, Seek};

use noodles_bgzf as bgzf;
use noodles_core::{region::Interval, Position};
use noodles_csi::{self as csi, index::reference_sequence::bin::Chunk};
use noodles_vcf as vcf;

use crate::header::StringMaps;

/// An iterator over records of a BCF reader that intersects a given region.
///
/// This is created by calling [`Reader::query`].
pub struct Query<'r, 'h, R>
where
    R: Read + Seek,
{
    reader: csi::io::Query<'r, R>,
    header: &'h vcf::Header,
    string_maps: &'r StringMaps,
    chromosome_id: usize,
    interval: Interval,
    buf: Vec<u8>,
    record: vcf::Record,
}

impl<'r, 'h, R> Query<'r, 'h, R>
where
    R: Read + Seek,
{
    pub(super) fn new(
        reader: &'r mut bgzf::Reader<R>,
        header: &'h vcf::Header,
        string_maps: &'r StringMaps,
        chunks: Vec<Chunk>,
        chromosome_id: usize,
        interval: Interval,
    ) -> Self {
        Self {
            reader: csi::io::Query::new(reader, chunks),
            header,
            string_maps,
            chromosome_id,
            interval,
            buf: Vec::new(),
            record: vcf::Record::default(),
        }
    }

    fn next_record(&mut self) -> io::Result<Option<vcf::Record>> {
        use super::read_record;

        read_record(
            &mut self.reader,
            self.header,
            self.string_maps,
            &mut self.buf,
            &mut self.record,
        )
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
    type Item = io::Result<vcf::Record>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.next_record() {
                Ok(Some(record)) => {
                    match intersects(self.string_maps, &record, self.chromosome_id, self.interval) {
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

fn intersects(
    string_maps: &StringMaps,
    record: &vcf::Record,
    chromosome_id: usize,
    region_interval: Interval,
) -> io::Result<bool> {
    let chromosome = record.chromosome();

    let id = string_maps
        .contigs()
        .get_index_of(&chromosome.to_string())
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("chromosome does not exist in contigs: {chromosome}"),
            )
        })?;

    let start = Position::try_from(usize::from(record.position()))
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    let end = record
        .end()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
        .map(usize::from)
        .and_then(|n| {
            Position::try_from(n).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
        })?;

    let record_interval = Interval::from(start..=end);

    Ok(id == chromosome_id && record_interval.intersects(region_interval))
}
