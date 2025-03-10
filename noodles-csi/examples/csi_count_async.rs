//! Prints the number of mapped records in the associated file.
//!
//! The result matches the output of `bcftools index --nrecords <src>`.

use std::env;

use noodles_csi::{self as csi, binning_index::ReferenceSequence};
use tokio::io;

#[tokio::main]
async fn main() -> io::Result<()> {
    let src = env::args().nth(1).expect("missing src");

    let csi_src = format!("{src}.csi");
    let index = csi::r#async::fs::read(csi_src).await?;

    let mut n = 0;

    for reference_sequence in index.reference_sequences() {
        if let Some(metadata) = reference_sequence.metadata() {
            n += metadata.mapped_record_count()
        }
    }

    println!("{n}");

    Ok(())
}
