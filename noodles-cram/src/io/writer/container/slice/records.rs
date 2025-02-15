use std::{
    collections::HashMap,
    error, fmt,
    io::{self, Write},
};

use bstr::BStr;
use noodles_core::Position;
use noodles_sam as sam;

use crate::{
    container::{
        block,
        compression_header::{data_series_encodings::DataSeries, preservation_map::tag_sets},
        CompressionHeader, ReferenceSequenceContext,
    },
    io::{
        writer::{record::Feature, Record},
        BitWriter,
    },
    record::{Flags, MateFlags},
};

#[allow(clippy::enum_variant_names)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WriteRecordError {
    MissingDataSeriesEncoding(DataSeries),
    MissingTagEncoding(tag_sets::Key),
}

impl error::Error for WriteRecordError {}

impl fmt::Display for WriteRecordError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingDataSeriesEncoding(data_series) => {
                write!(f, "missing data series encoding: {data_series:?}")
            }
            Self::MissingTagEncoding(key) => write!(f, "missing tag encoding: {key:?}"),
        }
    }
}

pub struct Writer<'a, W, X> {
    compression_header: &'a CompressionHeader,
    core_data_writer: &'a mut BitWriter<W>,
    external_data_writers: &'a mut HashMap<block::ContentId, X>,
    reference_sequence_context: ReferenceSequenceContext,
    prev_alignment_start: Option<Position>,
}

impl<'a, W, X> Writer<'a, W, X>
where
    W: Write,
    X: Write,
{
    pub fn new(
        compression_header: &'a CompressionHeader,
        core_data_writer: &'a mut BitWriter<W>,
        external_data_writers: &'a mut HashMap<block::ContentId, X>,
        reference_sequence_context: ReferenceSequenceContext,
    ) -> Self {
        let initial_alignment_start = match reference_sequence_context {
            ReferenceSequenceContext::Some(context) => Some(context.alignment_start()),
            _ => None,
        };

        Self {
            compression_header,
            core_data_writer,
            external_data_writers,
            reference_sequence_context,
            prev_alignment_start: initial_alignment_start,
        }
    }

    pub fn write_record(&mut self, record: &Record) -> io::Result<()> {
        self.write_bam_flags(record.bam_flags)?;
        self.write_cram_flags(record.cram_flags)?;

        self.write_positions(record)?;
        self.write_names(record)?;
        self.write_mate(record)?;

        self.write_tags(record)?;

        if record.bam_flags.is_unmapped() {
            self.write_unmapped_read(record)?;
        } else {
            self.write_mapped_read(record)?;
        }

        self.prev_alignment_start = record.alignment_start;

        Ok(())
    }

    fn write_bam_flags(&mut self, bam_flags: sam::alignment::record::Flags) -> io::Result<()> {
        let n = i32::from(u16::from(bam_flags));

        self.compression_header
            .data_series_encodings()
            .bam_flags()
            .ok_or_else(|| missing_data_series_encoding_error(DataSeries::BamFlags))?
            .encode(self.core_data_writer, self.external_data_writers, n)
    }

    fn write_cram_flags(&mut self, flags: Flags) -> io::Result<()> {
        let n = i32::from(u8::from(flags));

        self.compression_header
            .data_series_encodings()
            .cram_flags()
            .ok_or_else(|| missing_data_series_encoding_error(DataSeries::CramFlags))?
            .encode(self.core_data_writer, self.external_data_writers, n)
    }

    fn write_positions(&mut self, record: &Record) -> io::Result<()> {
        if self.reference_sequence_context.is_many() {
            self.write_reference_sequence_id(record.reference_sequence_id)?;
        }

        self.write_read_length(record.read_length)?;
        self.write_alignment_start(record.alignment_start)?;
        self.write_read_group_id(record.read_group_id)?;

        Ok(())
    }

    fn write_reference_sequence_id(
        &mut self,
        reference_sequence_id: Option<usize>,
    ) -> io::Result<()> {
        const UNMAPPED: i32 = -1;

        let encoding = self
            .compression_header
            .data_series_encodings()
            .reference_sequence_ids()
            .ok_or_else(|| missing_data_series_encoding_error(DataSeries::ReferenceSequenceIds))?;

        let n = if let Some(id) = reference_sequence_id {
            i32::try_from(id).map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?
        } else {
            UNMAPPED
        };

        encoding.encode(self.core_data_writer, self.external_data_writers, n)
    }

    fn write_read_length(&mut self, read_length: usize) -> io::Result<()> {
        let encoding = self
            .compression_header
            .data_series_encodings()
            .read_lengths()
            .ok_or_else(|| missing_data_series_encoding_error(DataSeries::ReadLengths))?;

        let len = i32::try_from(read_length)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        encoding.encode(self.core_data_writer, self.external_data_writers, len)
    }

    fn write_alignment_start(&mut self, alignment_start: Option<Position>) -> io::Result<()> {
        const MISSING: i32 = 0;

        fn position_to_i32(position: Position) -> io::Result<i32> {
            i32::try_from(usize::from(position))
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))
        }

        let ap_data_series_delta = self
            .compression_header
            .preservation_map()
            .ap_data_series_delta();

        let encoding = self
            .compression_header
            .data_series_encodings()
            .alignment_starts()
            .ok_or_else(|| missing_data_series_encoding_error(DataSeries::AlignmentStarts))?;

        let alignment_start_or_delta = if ap_data_series_delta {
            let start = alignment_start
                .map(position_to_i32)
                .transpose()?
                .unwrap_or(MISSING);

            let prev_start = self
                .prev_alignment_start
                .map(position_to_i32)
                .transpose()?
                .unwrap_or(MISSING);

            start - prev_start
        } else if let Some(position) = alignment_start {
            position_to_i32(position)?
        } else {
            MISSING
        };

        encoding.encode(
            self.core_data_writer,
            self.external_data_writers,
            alignment_start_or_delta,
        )
    }

    fn write_read_group_id(&mut self, read_group_id: Option<usize>) -> io::Result<()> {
        // § 10.2 "CRAM positional data" (2021-10-15): "-1 for no group".
        const MISSING: i32 = -1;

        let encoding = self
            .compression_header
            .data_series_encodings()
            .read_group_ids()
            .ok_or_else(|| missing_data_series_encoding_error(DataSeries::ReadGroupIds))?;

        let n = if let Some(id) = read_group_id {
            i32::try_from(id).map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?
        } else {
            MISSING
        };

        encoding.encode(self.core_data_writer, self.external_data_writers, n)
    }

    fn write_names(&mut self, record: &Record) -> io::Result<()> {
        let preservation_map = self.compression_header.preservation_map();

        if preservation_map.read_names_included() {
            let name = record.name.as_ref().map(|s| s.as_ref());
            self.write_name(name)?;
        }

        Ok(())
    }

    fn write_name(&mut self, name: Option<&BStr>) -> io::Result<()> {
        const MISSING: &[u8] = &[b'*', 0x00];

        let buf = name.map(|name| name.as_ref()).unwrap_or(MISSING);

        self.compression_header
            .data_series_encodings()
            .names()
            .ok_or_else(|| missing_data_series_encoding_error(DataSeries::Names))?
            .encode(self.core_data_writer, self.external_data_writers, buf)
    }

    fn write_mate(&mut self, record: &Record) -> io::Result<()> {
        if record.cram_flags.is_detached() {
            self.write_next_mate_bit_flags(record.mate_flags)?;

            let preservation_map = self.compression_header.preservation_map();

            if !preservation_map.read_names_included() {
                let name = record.name.as_ref().map(|s| s.as_ref());
                self.write_name(name)?;
            }

            self.write_next_fragment_reference_sequence_id(record.mate_reference_sequence_id)?;

            self.write_next_mate_alignment_start(record.mate_alignment_start)?;
            self.write_template_size(record.template_length)?;
        } else if let Some(distance_to_mate) = record.distance_to_mate {
            self.write_mate_distance(distance_to_mate)?;
        }

        Ok(())
    }

    fn write_next_mate_bit_flags(&mut self, next_mate_flags: MateFlags) -> io::Result<()> {
        let next_mate_bit_flags = i32::from(u8::from(next_mate_flags));

        self.compression_header
            .data_series_encodings()
            .mate_flags()
            .ok_or_else(|| missing_data_series_encoding_error(DataSeries::MateFlags))?
            .encode(
                self.core_data_writer,
                self.external_data_writers,
                next_mate_bit_flags,
            )
    }

    fn write_next_fragment_reference_sequence_id(
        &mut self,
        next_fragment_reference_sequence_id: Option<usize>,
    ) -> io::Result<()> {
        const UNMAPPED: i32 = -1;

        let encoding = self
            .compression_header
            .data_series_encodings()
            .mate_reference_sequence_ids()
            .ok_or_else(|| {
                missing_data_series_encoding_error(DataSeries::MateReferenceSequenceId)
            })?;

        let raw_next_fragment_reference_sequence_id =
            if let Some(id) = next_fragment_reference_sequence_id {
                i32::try_from(id).map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?
            } else {
                UNMAPPED
            };

        encoding.encode(
            self.core_data_writer,
            self.external_data_writers,
            raw_next_fragment_reference_sequence_id,
        )
    }

    fn write_next_mate_alignment_start(
        &mut self,
        next_mate_alignment_start: Option<Position>,
    ) -> io::Result<()> {
        let encoding = self
            .compression_header
            .data_series_encodings()
            .mate_alignment_starts()
            .ok_or_else(|| missing_data_series_encoding_error(DataSeries::MateAlignmentStart))?;

        let position = i32::try_from(
            next_mate_alignment_start
                .map(usize::from)
                .unwrap_or_default(),
        )
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

        encoding.encode(self.core_data_writer, self.external_data_writers, position)
    }

    fn write_template_size(&mut self, template_size: i32) -> io::Result<()> {
        self.compression_header
            .data_series_encodings()
            .template_lengths()
            .ok_or_else(|| missing_data_series_encoding_error(DataSeries::TemplateLengths))?
            .encode(
                self.core_data_writer,
                self.external_data_writers,
                template_size,
            )
    }

    fn write_mate_distance(&mut self, distance: usize) -> io::Result<()> {
        let encoding = self
            .compression_header
            .data_series_encodings()
            .mate_distances()
            .ok_or_else(|| missing_data_series_encoding_error(DataSeries::MateDistances))?;

        let n =
            i32::try_from(distance).map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

        encoding.encode(self.core_data_writer, self.external_data_writers, n)
    }

    fn write_tags(&mut self, record: &Record) -> io::Result<()> {
        use noodles_bam::record::codec::encoder::data::field::write_value;

        let tag_set: Vec<_> = record
            .data
            .iter()
            .map(|(tag, value)| tag_sets::Key::new(*tag, value.ty()))
            .collect();

        let tag_set_id = self
            .compression_header
            .preservation_map()
            .tag_sets()
            .iter()
            .position(|set| **set == tag_set)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing tag set"))?;

        self.write_tag_set_id(tag_set_id)?;

        let tag_encodings = self.compression_header.tag_encodings();
        let mut buf = Vec::new();

        for (key, (_, value)) in tag_set.into_iter().zip(&record.data) {
            let block_content_id = block::ContentId::from(key);

            write_value(&mut buf, &value.into())?;

            tag_encodings
                .get(&block_content_id)
                .ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        WriteRecordError::MissingTagEncoding(key),
                    )
                })?
                .encode(self.core_data_writer, self.external_data_writers, &buf)?;

            buf.clear();
        }

        Ok(())
    }

    fn write_tag_set_id(&mut self, tag_line: usize) -> io::Result<()> {
        let n =
            i32::try_from(tag_line).map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

        self.compression_header
            .data_series_encodings()
            .tag_set_ids()
            .ok_or_else(|| missing_data_series_encoding_error(DataSeries::TagSetIds))?
            .encode(self.core_data_writer, self.external_data_writers, n)
    }

    fn write_mapped_read(&mut self, record: &Record) -> io::Result<()> {
        self.write_number_of_read_features(record.features.len())?;

        let mut prev_position = 0;

        for feature in &record.features {
            let position = usize::from(feature.position()) - prev_position;
            self.write_feature(feature, position)?;
            prev_position = usize::from(feature.position());
        }

        self.write_mapping_quality(record.mapping_quality)?;

        if record.cram_flags.are_quality_scores_stored_as_array() {
            self.write_quality_scores(&record.quality_scores)?;
        }

        Ok(())
    }

    fn write_number_of_read_features(&mut self, feature_count: usize) -> io::Result<()> {
        let encoding = self
            .compression_header
            .data_series_encodings()
            .feature_counts()
            .ok_or_else(|| missing_data_series_encoding_error(DataSeries::FeatureCounts))?;

        let number_of_read_features = i32::try_from(feature_count)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

        encoding.encode(
            self.core_data_writer,
            self.external_data_writers,
            number_of_read_features,
        )
    }

    fn write_feature(&mut self, feature: &Feature, position: usize) -> io::Result<()> {
        self.write_feature_code(feature.code())?;
        self.write_feature_position(position)?;

        match feature {
            Feature::Bases { bases, .. } => {
                self.write_stretches_of_bases(bases)?;
            }
            Feature::Scores { quality_scores, .. } => {
                self.write_stretches_of_quality_scores(quality_scores)?;
            }
            Feature::ReadBase {
                base,
                quality_score,
                ..
            } => {
                self.write_base(*base)?;
                self.write_quality_score(*quality_score)?;
            }
            Feature::Substitution {
                reference_base,
                read_base,
                ..
            } => self.write_base_substitution_code(*reference_base, *read_base)?,
            Feature::Insertion { bases, .. } => {
                self.write_insertion(bases)?;
            }
            Feature::Deletion { len, .. } => {
                self.write_deletion_length(*len)?;
            }
            Feature::InsertBase { base, .. } => {
                self.write_base(*base)?;
            }
            Feature::QualityScore { quality_score, .. } => {
                self.write_quality_score(*quality_score)?;
            }
            Feature::ReferenceSkip { len, .. } => {
                self.write_reference_skip_length(*len)?;
            }
            Feature::SoftClip { bases, .. } => {
                self.write_soft_clip(bases)?;
            }
            Feature::Padding { len, .. } => {
                self.write_padding(*len)?;
            }
            Feature::HardClip { len, .. } => {
                self.write_hard_clip(*len)?;
            }
        }

        Ok(())
    }

    fn write_feature_code(&mut self, code: u8) -> io::Result<()> {
        self.compression_header
            .data_series_encodings()
            .feature_codes()
            .ok_or_else(|| missing_data_series_encoding_error(DataSeries::FeatureCodes))?
            .encode(self.core_data_writer, self.external_data_writers, code)
    }

    fn write_feature_position(&mut self, position: usize) -> io::Result<()> {
        let encoding = self
            .compression_header
            .data_series_encodings()
            .feature_position_deltas()
            .ok_or_else(|| missing_data_series_encoding_error(DataSeries::FeaturePositionDeltas))?;

        let position =
            i32::try_from(position).map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

        encoding.encode(self.core_data_writer, self.external_data_writers, position)
    }

    fn write_stretches_of_bases(&mut self, bases: &[u8]) -> io::Result<()> {
        self.compression_header
            .data_series_encodings()
            .stretches_of_bases()
            .ok_or_else(|| missing_data_series_encoding_error(DataSeries::StretchesOfBases))?
            .encode(self.core_data_writer, self.external_data_writers, bases)
    }

    fn write_stretches_of_quality_scores(&mut self, quality_scores: &[u8]) -> io::Result<()> {
        self.compression_header
            .data_series_encodings()
            .stretches_of_quality_scores()
            .ok_or_else(|| {
                missing_data_series_encoding_error(DataSeries::StretchesOfQualityScores)
            })?
            .encode(
                self.core_data_writer,
                self.external_data_writers,
                quality_scores,
            )
    }

    fn write_base(&mut self, base: u8) -> io::Result<()> {
        self.compression_header
            .data_series_encodings()
            .bases()
            .ok_or_else(|| missing_data_series_encoding_error(DataSeries::Bases))?
            .encode(self.core_data_writer, self.external_data_writers, base)
    }

    fn write_quality_score(&mut self, quality_score: u8) -> io::Result<()> {
        self.compression_header
            .data_series_encodings()
            .quality_scores()
            .ok_or_else(|| missing_data_series_encoding_error(DataSeries::QualityScores))?
            .encode(
                self.core_data_writer,
                self.external_data_writers,
                quality_score,
            )
    }

    fn write_base_substitution_code(
        &mut self,
        reference_base: u8,
        read_base: u8,
    ) -> io::Result<()> {
        use crate::record::feature::substitution::Base;

        let encoding = self
            .compression_header
            .data_series_encodings()
            .base_substitution_codes()
            .ok_or_else(|| missing_data_series_encoding_error(DataSeries::BaseSubstitutionCodes))?;

        let substitution_matrix = self
            .compression_header
            .preservation_map()
            .substitution_matrix();

        let reference_base = Base::try_from(reference_base)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

        let read_base = Base::try_from(read_base)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

        let code = substitution_matrix.find_code(reference_base, read_base);

        encoding.encode(self.core_data_writer, self.external_data_writers, code)
    }

    fn write_insertion(&mut self, bases: &[u8]) -> io::Result<()> {
        self.compression_header
            .data_series_encodings()
            .insertion_bases()
            .ok_or_else(|| missing_data_series_encoding_error(DataSeries::InsertionBases))?
            .encode(self.core_data_writer, self.external_data_writers, bases)
    }

    fn write_deletion_length(&mut self, len: usize) -> io::Result<()> {
        let encoding = self
            .compression_header
            .data_series_encodings()
            .deletion_lengths()
            .ok_or_else(|| missing_data_series_encoding_error(DataSeries::DeletionLengths))?;

        let n = i32::try_from(len).map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

        encoding.encode(self.core_data_writer, self.external_data_writers, n)
    }

    fn write_reference_skip_length(&mut self, len: usize) -> io::Result<()> {
        let encoding = self
            .compression_header
            .data_series_encodings()
            .reference_skip_lengths()
            .ok_or_else(|| missing_data_series_encoding_error(DataSeries::ReferenceSkipLengths))?;

        let n = i32::try_from(len).map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

        encoding.encode(self.core_data_writer, self.external_data_writers, n)
    }

    fn write_soft_clip(&mut self, bases: &[u8]) -> io::Result<()> {
        self.compression_header
            .data_series_encodings()
            .soft_clip_bases()
            .ok_or_else(|| missing_data_series_encoding_error(DataSeries::SoftClipBases))?
            .encode(self.core_data_writer, self.external_data_writers, bases)
    }

    fn write_padding(&mut self, len: usize) -> io::Result<()> {
        let encoding = self
            .compression_header
            .data_series_encodings()
            .padding_lengths()
            .ok_or_else(|| missing_data_series_encoding_error(DataSeries::PaddingLengths))?;

        let n = i32::try_from(len).map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

        encoding.encode(self.core_data_writer, self.external_data_writers, n)
    }

    fn write_hard_clip(&mut self, len: usize) -> io::Result<()> {
        let encoding = self
            .compression_header
            .data_series_encodings()
            .hard_clip_lengths()
            .ok_or_else(|| missing_data_series_encoding_error(DataSeries::HardClipLengths))?;

        let n = i32::try_from(len).map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

        encoding.encode(self.core_data_writer, self.external_data_writers, n)
    }

    fn write_mapping_quality(
        &mut self,
        mapping_quality: Option<sam::alignment::record::MappingQuality>,
    ) -> io::Result<()> {
        const MISSING: u8 = 0xff;

        let encoding = self
            .compression_header
            .data_series_encodings()
            .mapping_qualities()
            .ok_or_else(|| missing_data_series_encoding_error(DataSeries::MappingQualities))?;

        let n = i32::from(mapping_quality.map(u8::from).unwrap_or(MISSING));
        encoding.encode(self.core_data_writer, self.external_data_writers, n)
    }

    fn write_unmapped_read(&mut self, record: &Record) -> io::Result<()> {
        self.write_sequence(&record.sequence)?;

        if record.cram_flags.are_quality_scores_stored_as_array() {
            self.write_quality_scores(&record.quality_scores)?;
        }

        Ok(())
    }

    fn write_sequence(&mut self, sequence: &[u8]) -> io::Result<()> {
        self.compression_header
            .data_series_encodings()
            .bases()
            .ok_or_else(|| missing_data_series_encoding_error(DataSeries::Bases))?
            .get()
            .encode_extend(self.core_data_writer, self.external_data_writers, sequence)
    }

    fn write_quality_scores(&mut self, quality_scores: &[u8]) -> io::Result<()> {
        self.compression_header
            .data_series_encodings()
            .quality_scores()
            .ok_or_else(|| missing_data_series_encoding_error(DataSeries::QualityScores))?
            .get()
            .encode_extend(
                self.core_data_writer,
                self.external_data_writers,
                quality_scores,
            )
    }
}

fn missing_data_series_encoding_error(data_series: DataSeries) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        WriteRecordError::MissingDataSeriesEncoding(data_series),
    )
}
