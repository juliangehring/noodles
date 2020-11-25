use std::{convert::TryInto, io};

use noodles_sam as sam;

use super::{Data, Record, ReferenceSequenceId};

impl Record {
    /// Converts a BAM record to a SAM record.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::io;
    /// use noodles_bam as bam;
    /// use noodles_sam as sam;
    ///
    /// let reference_sequences = sam::header::ReferenceSequences::default();
    ///
    /// let record = bam::Record::default();
    /// let sam_record = record.try_into_sam_record(&reference_sequences)?;
    ///
    /// assert_eq!(sam_record, sam::Record::default());
    /// # Ok::<(), io::Error>(())
    /// ```
    pub fn try_into_sam_record(
        &self,
        reference_sequences: &sam::header::ReferenceSequences,
    ) -> io::Result<sam::Record> {
        let mut builder = sam::Record::builder();

        let raw_read_name = self
            .read_name()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))
            .and_then(|c_read_name| {
                c_read_name
                    .to_str()
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))
            })?;

        if raw_read_name != "*" {
            let read_name = raw_read_name
                .parse()
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

            builder = builder.set_read_name(read_name);
        }

        builder = builder.set_flags(self.flags());

        if let Some(reference_sequence_name) =
            get_reference_sequence_name(reference_sequences, self.reference_sequence_id())?
        {
            builder = builder.set_reference_sequence_name(reference_sequence_name);
        }

        if let Some(reference_sequence_id) = self.reference_sequence_id() {
            let id = i32::from(reference_sequence_id);

            let reference_sequence_name = reference_sequences
                .get_index(id as usize)
                .and_then(|(_, rs)| rs.name().parse().ok())
                .ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidInput, "invalid reference sequence ID")
                })?;

            builder = builder.set_reference_sequence_name(reference_sequence_name);
        }

        if let Some(position) = self.position() {
            builder = builder.set_position(position);
        }

        builder = builder
            .set_mapping_quality(self.mapping_quality())
            .set_cigar(self.cigar().try_into()?);

        if let Some(mate_reference_sequence_name) =
            get_reference_sequence_name(reference_sequences, self.mate_reference_sequence_id())?
        {
            builder = builder.set_mate_reference_sequence_name(mate_reference_sequence_name);
        }

        if let Some(mate_position) = self.mate_position() {
            builder = builder.set_mate_position(mate_position);
        }

        builder = builder.set_template_length(self.template_length());

        let raw_sequence = self.sequence().to_string();

        if !raw_sequence.is_empty() {
            let sequence = raw_sequence
                .parse()
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

            builder = builder.set_sequence(sequence);
        }

        let raw_quality_scores = self.quality_scores().chars().collect::<String>();

        if !raw_quality_scores.is_empty() {
            let quality_scores = raw_quality_scores
                .parse()
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

            builder = builder.set_quality_scores(quality_scores);
        }

        let data = convert_bam_data_to_sam_data(&self.data())?;
        builder = builder.set_data(data);

        Ok(builder.build())
    }
}

fn get_reference_sequence_name(
    reference_sequences: &sam::header::ReferenceSequences,
    reference_sequence_id: Option<ReferenceSequenceId>,
) -> io::Result<Option<sam::record::ReferenceSequenceName>> {
    reference_sequence_id
        .map(i32::from)
        .map(|id| {
            reference_sequences
                .get_index(id as usize)
                .and_then(|(_, rs)| rs.name().parse().ok())
                .ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidInput, "invalid reference sequence ID")
                })
        })
        .transpose()
}

fn convert_bam_data_to_sam_data(data: &Data<'_>) -> io::Result<sam::record::Data> {
    let mut sam_fields = Vec::new();

    for result in data.fields() {
        use noodles_sam::record::data::field::Value as SamValue;

        use crate::record::data::field::Value as BamValue;

        let bam_field = result?;
        let tag = bam_field.tag();

        let value = match bam_field.value() {
            BamValue::Char(c) => SamValue::Char(*c),
            BamValue::Int8(n) => SamValue::Int32(*n as i32),
            BamValue::UInt8(n) => SamValue::Int32(*n as i32),
            BamValue::Int16(n) => SamValue::Int32(*n as i32),
            BamValue::UInt16(n) => SamValue::Int32(*n as i32),
            BamValue::Int32(n) => SamValue::Int32(*n),
            // FIXME: lossy conversion
            BamValue::UInt32(n) => SamValue::Int32(*n as i32),
            BamValue::Float(n) => SamValue::Float(*n),
            BamValue::String(s) => SamValue::String(s.clone()),
            BamValue::Hex(s) => SamValue::Hex(s.clone()),
            BamValue::Int8Array(values) => SamValue::Int8Array(values.clone()),
            BamValue::UInt8Array(values) => SamValue::UInt8Array(values.clone()),
            BamValue::Int16Array(values) => SamValue::Int16Array(values.clone()),
            BamValue::UInt16Array(values) => SamValue::UInt16Array(values.clone()),
            BamValue::Int32Array(values) => SamValue::Int32Array(values.clone()),
            BamValue::UInt32Array(values) => SamValue::UInt32Array(values.clone()),
            BamValue::FloatArray(values) => SamValue::FloatArray(values.clone()),
        };

        let sam_field = sam::record::data::Field::new(tag.clone(), value);
        sam_fields.push(sam_field);
    }

    Ok(sam::record::Data::from(sam_fields))
}

#[cfg(test)]
mod tests {
    use std::{
        convert::TryFrom,
        ffi::CString,
        io::{self, BufWriter, Write},
    };

    use byteorder::{LittleEndian, WriteBytesExt};

    use super::*;

    fn build_reference_sequences() -> sam::header::ReferenceSequences {
        vec![
            (
                String::from("sq0"),
                sam::header::ReferenceSequence::new(String::from("sq0"), 5),
            ),
            (
                String::from("sq1"),
                sam::header::ReferenceSequence::new(String::from("sq1"), 8),
            ),
            (
                String::from("sq2"),
                sam::header::ReferenceSequence::new(String::from("sq2"), 13),
            ),
        ]
        .into_iter()
        .collect()
    }

    fn build_record() -> io::Result<Record> {
        let read_name =
            CString::new("r0").map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
        let flag = u16::from(sam::record::Flags::PAIRED | sam::record::Flags::READ_1);

        let mut writer = BufWriter::new(Vec::new());

        // ref_id
        writer.write_i32::<LittleEndian>(1)?;
        // pos
        writer.write_i32::<LittleEndian>(61061)?;
        // l_read_name
        writer.write_u8(read_name.as_bytes_with_nul().len() as u8)?;
        // mapq
        writer.write_u8(12)?;
        // bin
        writer.write_u16::<LittleEndian>(4684)?;
        // n_ciar_op
        writer.write_u16::<LittleEndian>(1)?;
        // flag
        writer.write_u16::<LittleEndian>(flag)?;
        // l_seq
        writer.write_u32::<LittleEndian>(4)?;
        // next_ref_id
        writer.write_i32::<LittleEndian>(1)?;
        // next_pos
        writer.write_i32::<LittleEndian>(61152)?;
        // tlen
        writer.write_i32::<LittleEndian>(166)?;
        // read_name
        writer.write_all(read_name.as_bytes_with_nul())?;
        // cigar
        writer.write_all(&[0x40, 0x00, 0x00, 0x00])?;
        // seq
        writer.write_all(&[0x18, 0x42])?;
        // qual
        writer.write_all(&[0x1f, 0x1d, 0x1e, 0x20])?;
        // data
        writer.write_all(&[
            0x4e, 0x4d, 0x43, 0x00, 0x50, 0x47, 0x5a, 0x53, 0x4e, 0x41, 0x50, 0x00,
        ])?;

        writer
            .into_inner()
            .map(Record::from)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))
    }

    #[test]
    fn test_try_into_sam_record() -> Result<(), Box<dyn std::error::Error>> {
        use sam::record::{
            cigar::{op, Op},
            data::{
                field::{Tag, Value},
                Field,
            },
        };

        let bam_record = build_record()?;
        let reference_sequences = build_reference_sequences();
        let actual = bam_record.try_into_sam_record(&reference_sequences)?;

        let expected = sam::Record::builder()
            .set_read_name("r0".parse()?)
            .set_flags(sam::record::Flags::PAIRED | sam::record::Flags::READ_1)
            .set_reference_sequence_name("sq1".parse()?)
            .set_position(sam::record::Position::try_from(61062)?)
            .set_mapping_quality(sam::record::MappingQuality::from(12))
            .set_cigar(sam::record::Cigar::from(vec![Op::new(op::Kind::Match, 4)]))
            .set_mate_reference_sequence_name("sq1".parse()?)
            .set_mate_position(sam::record::Position::try_from(61153)?)
            .set_template_length(166)
            .set_sequence("ATGC".parse()?)
            .set_quality_scores("@>?A".parse()?)
            .set_data(sam::record::Data::from(vec![
                Field::new(Tag::EditDistance, Value::Int32(0)),
                Field::new(Tag::Program, Value::String(String::from("SNAP"))),
            ]))
            .build();

        assert_eq!(actual, expected);

        Ok(())
    }
}
