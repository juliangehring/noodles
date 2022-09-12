use std::io::{self, Write};

use byteorder::WriteBytesExt;

use super::{Flags, Model, RangeCoder};
use crate::writer::num::write_uint7;

#[allow(dead_code)]
pub fn encode(flags: Flags, src: &[u8]) -> io::Result<Vec<u8>> {
    use crate::codecs::rans_nx16::encode::encode_pack;

    let mut src = src.to_vec();
    let mut dst = Vec::new();

    dst.write_u8(u8::from(flags))?;

    if !flags.contains(Flags::NO_SIZE) {
        let ulen =
            u32::try_from(src.len()).map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
        write_uint7(&mut dst, ulen)?;
    }

    if flags.contains(Flags::STRIPE) {
        todo!("encode_stripe");
    }

    let mut pack_header = None;

    if flags.contains(Flags::PACK) {
        let (header, buf) = encode_pack(&src)?;
        pack_header = Some(header);
        src = buf;
    }

    if let Some(header) = pack_header {
        dst.write_all(&header)?;
    }

    if flags.contains(Flags::CAT) {
        dst.write_all(&src)?;
    } else if flags.contains(Flags::EXT) {
        encode_ext(&src, &mut dst)?;
    } else if flags.contains(Flags::RLE) {
        if flags.contains(Flags::ORDER) {
            todo!("encode_rle_1");
        } else {
            todo!("encode_rle_0");
        }
    } else if flags.contains(Flags::ORDER) {
        encode_order_1(&src, &mut dst)?;
    } else {
        encode_order_0(&src, &mut dst)?;
    }

    Ok(dst)
}

fn encode_ext(src: &[u8], dst: &mut Vec<u8>) -> io::Result<()> {
    use bzip2::write::BzEncoder;

    let mut encoder = BzEncoder::new(dst, Default::default());
    encoder.write_all(src)?;
    encoder.finish()?;

    Ok(())
}

fn encode_order_0(src: &[u8], dst: &mut Vec<u8>) -> io::Result<()> {
    let max_sym = src.iter().max().copied().unwrap_or(0);
    dst.write_u8(max_sym + 1)?;

    let mut model = Model::new(max_sym);
    let mut range_coder = RangeCoder::default();

    for &sym in src {
        model.encode(dst, &mut range_coder, sym)?;
    }

    range_coder.range_encode_end(dst)?;

    Ok(())
}

fn encode_order_1(src: &[u8], dst: &mut Vec<u8>) -> io::Result<()> {
    let max_sym = src.iter().max().copied().unwrap_or(0);
    dst.write_u8(max_sym + 1)?;

    let model_count = usize::from(max_sym) + 1;
    let mut models = vec![Model::new(max_sym); model_count];

    let mut range_coder = RangeCoder::default();

    models[0].encode(dst, &mut range_coder, src[0])?;

    for window in src.windows(2) {
        let sym_0 = usize::from(window[0]);
        let sym_1 = window[1];
        models[sym_0].encode(dst, &mut range_coder, sym_1)?;
    }

    range_coder.range_encode_end(dst)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_ext() -> io::Result<()> {
        use crate::codecs::bzip2;

        let actual = encode(Flags::EXT, b"noodles")?;

        let mut expected = vec![0x04, 0x07];
        let data = bzip2::encode(b"noodles")?;
        expected.extend(data);

        assert_eq!(actual, expected);

        Ok(())
    }

    #[test]
    fn test_encode_order_0() -> io::Result<()> {
        let actual = encode(Flags::empty(), b"noodles")?;

        let expected = [
            0x00, 0x07, 0x74, 0x00, 0xf4, 0xe5, 0xb7, 0x4e, 0x50, 0x0f, 0x2e, 0x97, 0x00,
        ];

        assert_eq!(actual, expected);

        Ok(())
    }

    #[test]
    fn test_encode_order_1() -> io::Result<()> {
        let actual = encode(Flags::ORDER, b"noodles")?;

        let expected = [
            0x01, 0x07, 0x74, 0x00, 0xf4, 0xe3, 0x83, 0x41, 0xe2, 0x9a, 0xef, 0x53, 0x50, 0x00,
        ];

        assert_eq!(actual, expected);

        Ok(())
    }

    #[test]
    fn test_encode_cat() -> io::Result<()> {
        let actual = encode(Flags::CAT, b"noodles")?;
        let expected = [0x20, 0x07, 0x6e, 0x6f, 0x6f, 0x64, 0x6c, 0x65, 0x73];
        assert_eq!(actual, expected);
        Ok(())
    }
}
