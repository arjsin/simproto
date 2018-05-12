mod caller;
mod codec;
mod frame;
mod handler;

use self::caller::Caller;
use self::handler::Handler;
use bytes::Bytes;
use futures::unsync::mpsc;
use futures::Future;
use std::io;
use tokio_io::{AsyncRead, AsyncWrite};

trait Dialog {
    fn dialog<F>(self, f: F) -> (Caller, Handler)
    where
        F: FnMut(Bytes) -> Box<Future<Item = Bytes, Error = io::Error>> + 'static;
}

impl<A> Dialog for A
where
    A: AsyncRead + AsyncWrite + 'static,
{
    fn dialog<F>(self, f: F) -> (Caller, Handler)
    where
        F: FnMut(Bytes) -> Box<Future<Item = Bytes, Error = io::Error>> + 'static,
    {
        let (tx, rx) = mpsc::channel(1);
        (Caller::new(tx), Handler::new(self, rx, f))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use bytes::Bytes;
    use futures::future::ok;
    use std::cell::Cell;
    use std::rc::Rc;
    use std::time::Duration;
    use tokio_core::reactor::Core;
    use tokio_timer;
    use tokio_uds::UnixStream;

    #[test]
    fn simple_call() {
        let mut core = Core::new().unwrap();
        let assert_count = Rc::new(Cell::new(0));
        let handle = core.handle();
        let (s1, s2) = UnixStream::pair(&handle).unwrap();
        let (caller_echo, fut_echo) = s1.dialog(|req| Box::new(ok(req)));
        let (caller_del, fut_del) = s2.dialog(|_| Box::new(ok(Bytes::new())));
        handle.spawn(fut_del.map_err(|_| ()));
        handle.spawn(fut_echo.map_err(|_| ()));
        let buf = Bytes::from(&b"asdf"[..]);
        let f1 = {
            let mut assert_count = assert_count.clone();
            caller_echo.call(buf.clone()).and_then(move |(resp, _)| {
                assert_eq!(resp, Bytes::new());
                assert_count.set(assert_count.get() + 1);
                ok(())
            })
        };
        let f2 = {
            let mut assert_count = assert_count.clone();
            caller_del.call(buf.clone()).and_then(move |(resp, _)| {
                assert_eq!(resp, buf);
                assert_count.set(assert_count.get() + 1);
                ok(())
            })
        };
        let _ = core.run(f1.join(f2)).unwrap();
        assert_eq!(assert_count.get(), 2);
    }

    #[test]
    fn caller_close() {
        use std::io::Cursor;
        let data: Vec<u8> = vec![];
        let buf = Cursor::new(data);
        let (caller, fut) = buf.dialog(|req| Box::new(ok(req)));
        drop(caller);
        fut.wait().unwrap();
    }

    #[test]
    fn outoforder_call() {
        let mut core = Core::new().unwrap();
        let handle = core.handle();
        let (s1, s2) = UnixStream::pair(&handle).unwrap();
        let mut order = 2u64;
        let timer = tokio_timer::wheel()
            .tick_duration(Duration::from_millis(1))
            .build();
        let (_caller_echo, fut_echo) = s1.dialog(move |mut req| {
            if order != 0 {
                let local_order = order;
                order = order.saturating_sub(1);
                let timer = timer
                    .sleep(Duration::from_millis(local_order * 2))
                    .map_err(|_| io::Error::new(io::ErrorKind::Other, "unexpected timer error"));
                let resp = timer.and_then(move |_| {
                    req.extend_from_slice(&[local_order as u8]);
                    ok(req)
                });
                Box::new(resp)
            } else {
                req.extend_from_slice(&[0u8]);
                let resp = ok(req);
                Box::new(resp)
            }
        });
        let (caller_del, fut_del) = s2.dialog(|_| Box::new(ok(Bytes::new())));
        handle.spawn(fut_del.map_err(|_| ()));
        handle.spawn(fut_echo.map_err(|_| ()));
        let buf = Bytes::from(&[1][..]);
        let f1 = caller_del
            .call(buf)
            .and_then(|(resp, _)| {
                let expected = Bytes::from(&[1, 2][..]);
                assert_eq!(resp, expected);
                ok(())
            })
            .map_err(|e| {
                println!("{}", e);
                e
            });
        let buf = Bytes::from(&[2][..]);
        let f2 = caller_del.call(buf).and_then(|(resp, _)| {
            let expected = Bytes::from(&[2, 1][..]);
            assert_eq!(resp, expected);
            ok(())
        });
        let _ = core.run(f1.join(f2)).unwrap();
        let buf = Bytes::from(&[3][..]);
        let f3 = caller_del.call(buf).and_then(|(resp, _)| {
            let expected = Bytes::from(&[3, 0][..]);
            assert_eq!(resp, expected);
            ok(())
        });
        let _ = core.run(f3).unwrap();
    }
}
