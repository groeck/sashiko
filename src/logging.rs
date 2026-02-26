use std::io::{self, Write};
use tracing_subscriber::fmt::MakeWriter;

pub struct IgnoreBrokenPipe<W>(pub W);

impl<'a, W> MakeWriter<'a> for IgnoreBrokenPipe<W>
where
    W: MakeWriter<'a> + 'a,
{
    type Writer = IgnoreBrokenPipeWriter<W::Writer>;

    fn make_writer(&'a self) -> Self::Writer {
        IgnoreBrokenPipeWriter(self.0.make_writer())
    }
}

pub struct IgnoreBrokenPipeWriter<W>(W);

impl<W: Write> Write for IgnoreBrokenPipeWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self.0.write(buf) {
            Err(e) if e.kind() == io::ErrorKind::BrokenPipe => Ok(buf.len()),
            res => res,
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self.0.flush() {
            Err(e) if e.kind() == io::ErrorKind::BrokenPipe => Ok(()),
            res => res,
        }
    }
}
