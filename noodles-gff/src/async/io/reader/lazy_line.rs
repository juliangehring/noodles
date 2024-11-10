use tokio::io::{self, AsyncBufRead};

use super::read_line;
use crate::lazy;

pub(super) async fn read_lazy_line<R>(reader: &mut R, line: &mut lazy::Line) -> io::Result<usize>
where
    R: AsyncBufRead + Unpin,
{
    let buf = &mut line.0;
    buf.clear();
    read_line(reader, buf).await
}
