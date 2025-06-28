use bytes::BytesMut;
use std::io::{self, Write};

/// Wrapper that implements Write for `BytesMut`
pub struct BytesMutWriter<'a>(&'a mut BytesMut);

// `byteorder::WriteBytesExt` is automatically implemented for any type that implements Write
impl Write for BytesMutWriter<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub trait BytesMutWriteExt {
    fn write_adapter(&mut self) -> BytesMutWriter<'_>;
}

impl BytesMutWriteExt for BytesMut {
    fn write_adapter(&mut self) -> BytesMutWriter<'_> {
        BytesMutWriter(self)
    }
}
