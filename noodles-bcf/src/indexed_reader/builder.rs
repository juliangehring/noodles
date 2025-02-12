use std::{
    ffi::{OsStr, OsString},
    fs::File,
    io,
    path::{Path, PathBuf},
};

use noodles_bgzf as bgzf;
use noodles_csi as csi;

use super::IndexedReader;

/// An indexed BCF reader.
#[derive(Default)]
pub struct Builder {
    index: Option<csi::Index>,
}

impl Builder {
    /// Builds an indexed BCF reader from a path.
    pub fn build_from_path<P>(self, src: P) -> io::Result<IndexedReader<bgzf::Reader<File>>>
    where
        P: AsRef<Path>,
    {
        let src = src.as_ref();

        let file = File::open(src)?;

        let index = match self.index {
            Some(index) => index,
            None => read_associated_index(src)?,
        };

        Ok(IndexedReader::new(file, index))
    }
}

fn read_associated_index<P>(src: P) -> io::Result<csi::Index>
where
    P: AsRef<Path>,
{
    csi::read(build_index_src(src))
}

fn build_index_src<P>(src: P) -> PathBuf
where
    P: AsRef<Path>,
{
    const EXT: &str = "csi";
    push_ext(src.as_ref().into(), EXT)
}

fn push_ext<S>(path: PathBuf, ext: S) -> PathBuf
where
    S: AsRef<OsStr>,
{
    let mut s = OsString::from(path);
    s.push(".");
    s.push(ext);
    PathBuf::from(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_index_src() {
        assert_eq!(
            build_index_src("sample.bcf"),
            PathBuf::from("sample.bcf.csi")
        );
    }
}
