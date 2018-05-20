mod caller;
mod codec;
mod frame;
mod handler;

use std::io;

pub use self::caller::Caller;
pub use self::codec::Codec;
pub use self::frame::{Frame, TypeLabel};
pub use self::handler::Handler;

use bytes::Bytes;
use futures::channel::mpsc;
use futures::io::{AsyncRead, AsyncWrite};
use futures::prelude::*;

pub trait Dialog {
    fn dialog<F>(self, f: F) -> (Caller, Handler)
    where
        F: FnMut(Caller, Bytes) -> Box<Future<Item = Bytes, Error = io::Error>> + 'static;
}

impl<A> Dialog for A
where
    A: AsyncRead + AsyncWrite + 'static,
{
    fn dialog<F>(self, f: F) -> (Caller, Handler)
    where
        F: FnMut(Caller, Bytes) -> Box<Future<Item = Bytes, Error = io::Error>> + 'static,
    {
        let (tx, rx) = mpsc::channel(1);
        let caller = Caller::new(tx);
        (caller.clone(), Handler::new(self, rx, caller, f))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use bytes::Bytes;
    use futures::executor::LocalPool;
    use futures::future::ok;
    use std::cell::Cell;
    use std::rc::Rc;
    use util::PairIO;

    #[test]
    fn simple_call() {
        let assert_count = Rc::new(Cell::new(0));
        let (s1, s2) = PairIO::new();
        let (caller_echo, fut_echo) = s1.dialog(|_, req| Box::new(ok(req)));
        let (caller_del, fut_del) = s2.dialog(|_, _| Box::new(ok(Bytes::new())));
        let mut pool = LocalPool::new();
        let mut executor = pool.executor();
        executor
            .spawn_local(fut_del.map_err(|_| panic!("fut_del panic")))
            .unwrap();
        executor
            .spawn_local(fut_echo.map_err(|_| panic!("fut_echo panic")))
            .unwrap();

        let buf = Bytes::from(&b"asdf"[..]);
        let f1 = {
            let mut assert_count = assert_count.clone();
            caller_echo.call(buf).and_then(move |(_, resp)| {
                assert_eq!(resp, Bytes::new());
                assert_count.set(assert_count.get() + 1);
                ok(())
            })
        };
        let buf = Bytes::from(&b"123"[..]);
        let f2 = {
            let mut assert_count = assert_count.clone();
            caller_del.call(buf.clone()).and_then(move |(_, resp)| {
                assert_eq!(resp, buf);
                assert_count.set(assert_count.get() + 1);
                ok(())
            })
        };
        let _ = pool.run_until(f1.join(f2), &mut executor).unwrap();
        assert_eq!(assert_count.get(), 2);
    }
}
