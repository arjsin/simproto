extern crate futures;

use futures::channel::mpsc::{channel, Receiver, Sender};
use futures::prelude::*;
use std::io::{self, Cursor};
use std::sync::{Arc, Mutex};

type InMemIO = Arc<Mutex<Cursor<Vec<u8>>>>;

pub struct OneEndIO {
    in_io: InMemIO,
    in_ch: Receiver<()>,
    out_io: InMemIO,
    out_ch: Sender<()>,
    in_pos: u64,
    flush: bool,
    read_pending: bool,
}

impl OneEndIO {
    fn new(in_io: InMemIO, out_io: InMemIO, in_ch: Receiver<()>, out_ch: Sender<()>) -> OneEndIO {
        OneEndIO {
            in_io,
            in_ch,
            out_io,
            out_ch,
            in_pos: 0,
            flush: false,
            read_pending: false,
        }
    }

    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut in_io = self.in_io.lock().unwrap();
        let old_pos = in_io.position();
        in_io.set_position(self.in_pos);
        let read = io::Read::read(&mut *in_io, buf);
        self.in_pos = in_io.position();
        if self.in_pos == old_pos {
            in_io.set_position(0);
            in_io.get_mut().clear();
            self.in_pos = 0;
            self.read_pending = false;
        } else {
            in_io.set_position(old_pos);
            self.read_pending = true;
        }
        read
    }
}

impl AsyncRead for OneEndIO {
    fn poll_read(
        &mut self,
        cx: &mut task::Context,
        buf: &mut [u8],
    ) -> Result<Async<usize>, io::Error> {
        if self.read_pending {
            match self.read(buf) {
                Ok(t) => Ok(Async::Ready(t)),
                Err(e) => return Err(e.into()),
            }
        } else {
            match self.in_ch.poll_next(cx) {
                Ok(Async::Ready(Some(()))) => match self.read(buf) {
                    Ok(t) => Ok(Async::Ready(t)),
                    Err(e) => return Err(e.into()),
                },
                Ok(Async::Ready(None)) => Ok(Async::Ready(0)),
                Ok(Async::Pending) => Ok(Async::Pending),
                Err(_) => Err(io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "unable to read channel",
                )),
            }
        }
    }
}

impl AsyncWrite for OneEndIO {
    fn poll_write(&mut self, _: &mut task::Context, buf: &[u8]) -> Result<Async<usize>, io::Error> {
        match io::Write::write(&mut *self.out_io.lock().unwrap(), buf) {
            Ok(x) => {
                self.flush = true;
                Ok(Async::Ready(x))
            }
            Err(e) => Err(e),
        }
    }

    fn poll_flush(&mut self, cx: &mut task::Context) -> Poll<(), io::Error> {
        if !self.flush {
            return Ok(Async::Ready(()));
        }
        // TODO: store states and fix flush boolean
        match self.out_ch.start_send(()) {
            Ok(()) => match self.out_ch.poll_flush(cx) {
                Ok(Async::Ready(())) => {
                    self.flush = false;
                    Ok(Async::Ready(()))
                }
                Ok(Async::Pending) => Ok(Async::Pending),
                Err(e) => if e.is_full() {
                    Ok(Async::Ready(()))
                } else {
                    Err(io::Error::new(
                        io::ErrorKind::BrokenPipe,
                        "unable to notify flush",
                    ))
                },
            },
            Err(e) => if e.is_full() {
                Ok(Async::Ready(()))
            } else {
                Err(io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "unable to notify flush",
                ))
            },
        }
    }

    fn poll_close(&mut self, cx: &mut task::Context) -> Poll<(), io::Error> {
        match self.out_ch.poll_close(cx) {
            Ok(x) => Ok(x),
            Err(_) => Err(io::Error::new(io::ErrorKind::Other, "unable to close")),
        }
    }
}

pub struct PairIO;

impl PairIO {
    pub fn new() -> (OneEndIO, OneEndIO) {
        let io1 = Arc::new(Mutex::new(Cursor::new(Vec::new())));
        let io2 = Arc::new(Mutex::new(Cursor::new(Vec::new())));
        let (out_ch1, in_ch1) = channel(0);
        let (out_ch2, in_ch2) = channel(0);
        (
            OneEndIO::new(io1.clone(), io2.clone(), in_ch1, out_ch2),
            OneEndIO::new(io2, io1, in_ch2, out_ch1),
        )
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use futures::executor::block_on;

    #[test]
    fn write_read() {
        let (s1, s2) = PairIO::new();
        let fut1 = s1
            .write_all(&[0u8, 1])
            .and_then(|(s1, _)| s1.flush())
            .and_then(|s1| s1.write_all(&[3]))
            .and_then(|(s1, _)| s1.flush())
            .and_then(|s1| s1.close());
        let buf = vec![];
        let buf = block_on(s2.read_to_end(buf).join(fut1).map(|((_, buf), _)| buf)).unwrap();
        assert_eq!(buf, [0u8, 1, 3]);
    }

    #[test]
    fn fill_buffer() {
        let (s1, s2) = PairIO::new();
        let fut1 = s1
            .write_all(&[0])
            .and_then(|(s, _)| s.flush())
            .and_then(|s| s.write_all(&[1]))
            .and_then(|(s, _)| s.flush())
            .and_then(|s| s.write_all(&[2]))
            .and_then(|(s, _)| s.flush())
            .and_then(|s| s.write_all(&[3]))
            .and_then(|(s, _)| s.flush())
            .and_then(|s| s.write_all(&[4]))
            .and_then(|(s, _)| s.flush())
            .and_then(|s| s.close());
        let buf = vec![];
        let buf = block_on(s2.read_to_end(buf).join(fut1).map(|((_, buf), _)| buf)).unwrap();
        assert_eq!(buf, [0u8, 1, 2, 3, 4]);
    }
}
