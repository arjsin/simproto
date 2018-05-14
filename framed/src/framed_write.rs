use std::io;

use codec::{Decoder, Encoder};

use bytes::BytesMut;
use futures::prelude::*;

pub struct FramedWrite2<T> {
    inner: T,
    buffer: BytesMut,
}

const INITIAL_CAPACITY: usize = 8 * 1024;
const BACKPRESSURE_BOUNDARY: usize = INITIAL_CAPACITY;

// ===== impl FramedWrite2 =====

pub fn framed_write2<T>(inner: T) -> FramedWrite2<T> {
    FramedWrite2 {
        inner: inner,
        buffer: BytesMut::with_capacity(INITIAL_CAPACITY),
    }
}

pub fn framed_write2_with_buffer<T>(inner: T, mut buf: BytesMut) -> FramedWrite2<T> {
    if buf.capacity() < INITIAL_CAPACITY {
        let bytes_to_reserve = INITIAL_CAPACITY - buf.capacity();
        buf.reserve(bytes_to_reserve);
    }
    FramedWrite2 {
        inner: inner,
        buffer: buf,
    }
}

impl<T> FramedWrite2<T> {
    pub fn get_ref(&self) -> &T {
        &self.inner
    }

    pub fn into_inner(self) -> T {
        self.inner
    }

    pub fn into_parts(self) -> (T, BytesMut) {
        (self.inner, self.buffer)
    }

    pub fn get_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}

impl<T> Sink for FramedWrite2<T>
where
    T: AsyncWrite + Encoder,
{
    type SinkItem = T::Item;
    type SinkError = T::Error;

    fn poll_ready(&mut self, cx: &mut task::Context) -> Result<Async<()>, Self::SinkError> {
        // If the buffer is already over 8KiB, then attempt to flush it. If after flushing it's
        // *still* over 8KiB, then apply backpressure (reject the send).
        if self.buffer.len() >= BACKPRESSURE_BOUNDARY {
            try!(self.poll_flush(cx));

            if self.buffer.len() >= BACKPRESSURE_BOUNDARY {
                Ok(Async::Pending)
            } else {
                Ok(Async::Ready(()))
            }
        } else {
            Ok(Async::Ready(()))
        }
    }

    fn start_send(&mut self, item: T::Item) -> Result<(), Self::SinkError> {
        try!(self.inner.encode(item, &mut self.buffer));

        Ok(())
    }

    fn poll_flush(&mut self, cx: &mut task::Context) -> Poll<(), Self::SinkError> {
        trace!("flushing framed transport");

        while !self.buffer.is_empty() {
            trace!("writing; remaining={}", self.buffer.len());

            let n = try_ready!(self.inner.poll_write(cx, &self.buffer));

            if n == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::WriteZero,
                    "failed to write frame to transport",
                ).into());
            }

            // TODO: Add a way to `bytes` to do this w/o returning the drained
            // data.
            let _ = self.buffer.split_to(n);
        }

        // Try flushing the underlying IO
        try_ready!(self.inner.poll_flush(cx));

        trace!("framed transport flushed");
        return Ok(Async::Ready(()));
    }

    fn poll_close(&mut self, cx: &mut task::Context) -> Poll<(), Self::SinkError> {
        try_ready!(self.poll_flush(cx));
        Ok(try!(self.inner.poll_close(cx)))
    }
}

impl<T: Decoder> Decoder for FramedWrite2<T> {
    type Item = T::Item;
    type Error = T::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<T::Item>, T::Error> {
        self.inner.decode(src)
    }

    fn decode_eof(&mut self, src: &mut BytesMut) -> Result<Option<T::Item>, T::Error> {
        self.inner.decode_eof(src)
    }
}

impl<T: AsyncRead> AsyncRead for FramedWrite2<T> {
    fn poll_read(&mut self, cx: &mut task::Context, buf: &mut [u8]) -> Result<Async<usize>, io::Error> {
        self.inner.poll_read(cx, buf)
    }
}
