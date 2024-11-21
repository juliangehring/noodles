//! Prints all lines in a GFF file, up to the FASTA section.
//!
//! Lines are parsed as either a directive, comment, or record.

use std::{
    env,
    fs::File,
    io::{self, BufReader},
};

use noodles_gff::{self as gff, directive_buf::key, DirectiveBuf, LineBuf};

fn main() -> io::Result<()> {
    let src = env::args().nth(1).expect("missing src");

    let mut reader = File::open(src)
        .map(BufReader::new)
        .map(gff::io::Reader::new)?;

    let stdout = io::stdout().lock();
    let mut writer = gff::io::Writer::new(stdout);

    for result in reader.line_bufs() {
        let line = result?;

        if matches!(line, LineBuf::Directive(DirectiveBuf::Other(ref key, _)) if key == key::START_OF_FASTA)
        {
            break;
        }

        writer.write_line(&line)?;
    }

    Ok(())
}
