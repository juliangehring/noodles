//! Replaces the header of a tabix-indexed file.
//!
//! The result matches the output of `tabix --reheader <header-src> <src>`.

use std::{
    env,
    fs::File,
    io::{self, BufRead, BufReader, Write},
};

use noodles_bgzf as bgzf;
use noodles_tabix as tabix;

const LINE_FEED: u8 = b'\n';

fn main() -> io::Result<()> {
    let mut args = env::args().skip(1);
    let header_src = args.next().expect("missing header-src");
    let src = args.next().expect("missing src");

    let tabix_src = format!("{}.tbi", src);
    let index = tabix::read(tabix_src)?;

    let stdout = io::stdout().lock();
    let mut writer = bgzf::Writer::new(stdout);

    let mut header_reader = File::open(header_src).map(BufReader::new)?;
    io::copy(&mut header_reader, &mut writer)?;

    let mut reader = File::open(src).map(bgzf::Reader::new)?;
    let line_comment_prefix = index.header().line_comment_prefix();

    let mut buf = Vec::new();

    loop {
        buf.clear();

        match reader.read_until(LINE_FEED, &mut buf) {
            Ok(0) => break,
            Ok(_) => {
                if !buf.starts_with(&[line_comment_prefix]) {
                    writer.write_all(&buf)?;
                    break;
                }
            }
            Err(e) => return Err(e),
        }
    }

    io::copy(&mut reader, &mut writer)?;

    Ok(())
}
