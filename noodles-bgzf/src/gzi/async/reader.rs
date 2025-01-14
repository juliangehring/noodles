mod index;

use tokio::io::{self, AsyncRead};

use self::index::read_index;
use crate::gzi::Index;

/// An async gzip index (GZI) reader.
pub struct Reader<R> {
    inner: R,
}

impl<R> Reader<R>
where
    R: AsyncRead + Unpin,
{
    /// Creates an async gzip index (GZI) reader.
    ///
    /// # Examples
    ///
    /// ```
    /// use noodles_bgzf::gzi;
    /// let data = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    /// let reader = gzi::r#async::Reader::new(&data[..]);
    /// ```
    pub fn new(inner: R) -> Self {
        Self { inner }
    }

    /// Reads a gzip index.
    ///
    /// The position of the stream is expected to be at the start.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use std::io;
    /// #
    /// # #[tokio::main]
    /// # async fn main() -> io::Result<()> {
    /// use noodles_bgzf::gzi;
    /// use tokio::fs::File;
    ///
    /// let mut reader = File::open("in.gzi")
    ///     .await
    ///     .map(gzi::r#async::Reader::new)?;
    ///
    /// let index = reader.read_index().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn read_index(&mut self) -> io::Result<Index> {
        read_index(&mut self.inner).await
    }
}
