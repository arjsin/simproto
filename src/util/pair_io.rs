extern crate futures;

use futures::channel::mpsc::{channel, Receiver, Sender};
use futures::prelude::*;
use std::cell::RefCell;
use std::io::{self, Cursor};
use std::rc::Rc;

type InMemIO = Rc<RefCell<Cursor<Vec<u8>>>>;

pub struct OneEndIO {
    in_io: InMemIO,
    in_ch: Receiver<()>,
    out_io: InMemIO,
    out_ch: Sender<()>,
    in_pos: u64,
}

impl OneEndIO {
    fn new(in_io: InMemIO, out_io: InMemIO, in_ch: Receiver<()>, out_ch: Sender<()>) -> OneEndIO {
        OneEndIO {
            in_io,
            in_ch,
            out_io,
            out_ch,
            in_pos: 0,
        }
    }

    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let old_pos = (*self.in_io).borrow().position();
        (*self.in_io).borrow_mut().set_position(self.in_pos);
        let read = io::Read::read(&mut *self.in_io.borrow_mut(), buf);
        self.in_pos = (*self.in_io).borrow().position();
        if self.in_pos == old_pos {
            (*self.in_io).borrow_mut().set_position(0);
            (*self.in_io).borrow_mut().get_mut().clear();
            self.in_pos = 0;
        } else {
            (*self.in_io).borrow_mut().set_position(old_pos);
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

impl AsyncWrite for OneEndIO {
    fn poll_write(&mut self, _: &mut task::Context, buf: &[u8]) -> Result<Async<usize>, io::Error> {
        match io::Write::write(&mut *self.out_io.borrow_mut(), buf) {
            Ok(x) => Ok(Async::Ready(x)),
            Err(e) => Err(e),
        }
    }

    fn poll_flush(&mut self, cx: &mut task::Context) -> Poll<(), io::Error> {
        // TODO: store states
        match self.out_ch.start_send(()) {
            Ok(()) => match self.out_ch.poll_flush(cx) {
                Ok(Async::Ready(())) => Ok(Async::Ready(())),
                Ok(Async::Pending) => Ok(Async::Pending),
                Err(_) => Err(io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "unable to notify flush",
                )),
            },
            Err(_) => Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "unable to notify flush",
            )),
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
        let io1 = Rc::new(RefCell::new(Cursor::new(Vec::new())));
        let io2 = Rc::new(RefCell::new(Cursor::new(Vec::new())));
        let (out_ch1, in_ch1) = channel(1);
        let (out_ch2, in_ch2) = channel(1);
        (
            OneEndIO::new(io1.clone(), io2.clone(), in_ch1, out_ch2),
            OneEndIO::new(io2, io1, in_ch2, out_ch1),
        )
    }
}

#[test]
fn write_read() {
    use futures::executor::block_on;
    use futures::prelude::*;

    let (s1, s2) = PairIO::new();
    let fut1 = s1.write_all(&[0u8, 1])
        .and_then(|(s1, _)| s1.flush())
        .and_then(|s1| s1.write_all(&[3]))
        .and_then(|(s1, _)| s1.flush())
        .and_then(|s1| s1.close());
    let buf = vec![];
    let buf = block_on(s2.read_to_end(buf).join(fut1).map(|((_, buf), _)| buf)).unwrap();
    assert_eq!(buf, [0u8, 1, 3]);
}
