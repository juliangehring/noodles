use tokio::io::{self, AsyncBufRead};

use crate::Line;

pub(super) async fn read_line<R>(reader: &mut R, line: &mut Line) -> io::Result<usize>
where
    R: AsyncBufRead + Unpin,
{
    use crate::io::reader::line::is_blank;

    let buf = &mut line.0;

    loop {
        buf.clear();

        let n = super::read_line(reader, buf).await?;

        if n == 0 || !is_blank(buf) {
            return Ok(n);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_read_line() -> io::Result<()> {
        const DATA: &[u8] = b"\n#comment\n\t\n";

        let mut line = Line::default();
        let mut lines: Vec<String> = Vec::new();

        let mut src = DATA;

        while read_line(&mut src, &mut line).await? != 0 {
            lines.push(line.as_ref().into());
        }

        assert_eq!(lines, [String::from("#comment")]);

        Ok(())
    }
}
