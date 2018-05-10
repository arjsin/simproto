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
        let (dialog_tx, dialog_rx) = dialog_io.framed(Codec).split();
        let (internal_tx, internal_rx) = mpsc::channel::<Frame>(1);

        let caller_resp_map: Rc<RefCell<HashMap<u64, oneshot::Sender<_>>>> =
            Rc::new(RefCell::new(HashMap::new()));

        // Receive requests and oneshot from dialog caller
        let caller_ch_fut = {
            let caller_resp_map = Rc::clone(&caller_resp_map);
            let internal_tx = internal_tx.clone();

            caller_ch
                .for_each(move |data| {
                    let internal_tx = internal_tx.clone();
                    let (id, oneshot, message) = data;
                    caller_resp_map.borrow_mut().insert(id, oneshot);
                    let req = Frame::new(TypeLabel::Request, id, message);
                    internal_tx.send(req).map(|_| ()).map_err(|_| ())
                })
                .map(|_| ())
                .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "receiver channel broken"))
        };

        // Receive requests and responses
        let dialog_rx_fut = dialog_rx.for_each(move |message| {
            let internal_tx = internal_tx.clone();
            Handler::receiver(message, internal_tx, &caller_resp_map, &mut f)
        });

        // Actual sending of data from internal channel
        let dialog_tx_fut =
            dialog_tx.send_all(internal_rx.map_err(|_| {
                io::Error::new(io::ErrorKind::Other, "unexpected sender channel error")
            }));

        // TODO: use Either::split for errors
        let future = dialog_tx_fut
            .map(|_| ())
            .select(dialog_rx_fut)
            .map(|_| ())
            .map_err(|(a, _)| a)
            .select(caller_ch_fut)
            .map(|_| ())
            .map_err(|(a, _)| a);
        Handler {
            f: Box::new(future),
        }
    }

    fn receiver<F>(
        message: Frame,
        sender: mpsc::Sender<Frame>,
        sender_map: &Rc<RefCell<HashMap<u64, oneshot::Sender<Bytes>>>>,
        f: &mut F,
    ) -> Box<Future<Item = (), Error = io::Error>>
    where
        F: FnMut(Bytes) -> Box<Future<Item = Bytes, Error = io::Error>>,
    {
        let (t, id, payload) = message.into();
        match t {
            TypeLabel::Request => {
                let send_fut = f(payload)
                    .and_then(move |resp| {
                        let resp = Frame::new(TypeLabel::Response, id, resp);
                        sender.send(resp).map_err(|_| {
                            io::Error::new(io::ErrorKind::BrokenPipe, "sender channel broken")
                        })
                    })
                    .map(|_| ());
                Box::new(send_fut)
            }
            TypeLabel::Response => {
                if let Some(mut c) = sender_map.borrow_mut().remove(&id) {
                    let _ = c.send(payload);
                }
                Box::new(ok(()))
            }
            TypeLabel::Ping => {
                let pong = Frame::new(TypeLabel::Pong, id, payload);
                let send_fut = sender
                    .send(pong)
                    .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "sender channel broken"))
                    .map(|_| ());
                Box::new(send_fut)
            }
            _ => Box::new(ok(())),
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
