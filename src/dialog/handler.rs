use super::codec::Codec;
use super::frame::{Frame, TypeLabel};
use bytes::Bytes;
use futures::future::ok;
use futures::unsync::{mpsc, oneshot};
use futures::{Future, Poll, Sink, Stream};
use std::cell::RefCell;
use std::collections::HashMap;
use std::io;
use std::rc::Rc;
use tokio_io::{AsyncRead, AsyncWrite};

pub struct Handler {
    f: Box<Future<Item = (), Error = io::Error>>,
}

impl Handler {
    pub fn new<F, A>(
        dialog_io: A,
        caller_ch: mpsc::Receiver<(u64, oneshot::Sender<Bytes>, Bytes)>,
        mut f: F,
    ) -> Handler
    where
        F: FnMut(Bytes) -> Box<Future<Item = Bytes, Error = io::Error>> + 'static,
        A: AsyncRead + AsyncWrite + 'static,
    {
        let (dialog_sink, dialog_stream) = dialog_io.framed(Codec).split();

        let caller_resp_map: Rc<RefCell<HashMap<u64, oneshot::Sender<_>>>> =
            Rc::new(RefCell::new(HashMap::new()));

        let caller_ch_stream = {
            let caller_resp_map = Rc::clone(&caller_resp_map);
            caller_ch
                .map(move |data| {
                    let (id, oneshot, message) = data;
                    caller_resp_map.borrow_mut().insert(id, oneshot);
                    Frame::new(TypeLabel::Request, id, message)
                })
                .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "caller channel broken"))
        };

        let dialog_stream = dialog_stream
            .and_then(move |message| Handler::receiver(message, &caller_resp_map, &mut f))
            .filter_map(|x| x);

        let fut = dialog_sink
            .send_all(caller_ch_stream.select(dialog_stream))
            .map(|_| ());

        Handler { f: Box::new(fut) }
    }

    fn receiver<F>(
        message: Frame,
        sender_map: &Rc<RefCell<HashMap<u64, oneshot::Sender<Bytes>>>>,
        f: &mut F,
    ) -> Box<Future<Item = Option<Frame>, Error = io::Error>>
    where
        F: FnMut(Bytes) -> Box<Future<Item = Bytes, Error = io::Error>>,
    {
        let (t, id, payload) = message.into();
        match t {
            TypeLabel::Request => {
                let send_fut =
                    f(payload).map(move |resp| Some(Frame::new(TypeLabel::Response, id, resp)));
                Box::new(send_fut)
            }
            TypeLabel::Response => {
                if let Some(mut c) = sender_map.borrow_mut().remove(&id) {
                    let _ = c.send(payload);
                }
                Box::new(ok(None))
            }
            TypeLabel::Ping => {
                let pong = Frame::new(TypeLabel::Pong, id, payload);
                Box::new(ok(Some(pong)))
            }
            _ => Box::new(ok(None)),
        }
    }
}

impl Future for Handler {
    type Item = ();
    type Error = io::Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.f.poll()
    }
}
