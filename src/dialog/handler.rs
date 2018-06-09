use std::collections::HashMap;
use std::io;
use std::sync::Arc;

use super::Codec;
use super::{Frame, TypeLabel};

use bytes::Bytes;
use framed::framed::framed;
use futures::channel::{mpsc, oneshot};
use futures::future::ok;
use futures::io::{AsyncRead, AsyncWrite};
use futures::prelude::*;
use parking_lot::Mutex;

pub struct Handler {
    f: Box<Future<Item = (), Error = io::Error> + Send + Sync>,
}

impl Handler {
    pub fn new<F, A>(
        dialog_io: A,
        caller_ch: mpsc::Receiver<(usize, oneshot::Sender<Bytes>, Bytes)>,
        mut f: F,
    ) -> Handler
    where
        F: FnMut(Bytes) -> Box<Future<Item = Bytes, Error = io::Error> + Send + Sync>,
        F: Send + Sync + 'static,
        A: AsyncRead + AsyncWrite + Send + Sync + 'static,
    {
        let (dialog_sink, dialog_stream) = framed(dialog_io, Codec).split();

        let caller_resp_map: Arc<Mutex<HashMap<usize, oneshot::Sender<_>>>> =
            Arc::new(Mutex::new(HashMap::new()));

        let caller_ch_stream = {
            let caller_resp_map = Arc::clone(&caller_resp_map);
            caller_ch
                .map(move |data| {
                    let (id, oneshot, message) = data;
                    caller_resp_map.lock().insert(id, oneshot);
                    Frame::new(TypeLabel::Request, id as u64, message)
                })
                .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "caller channel broken"))
        };

        let dialog_stream = dialog_stream
            .and_then(move |message| Handler::receiver(message, &caller_resp_map, &mut f))
            .filter_map(|x| Ok(x));

        let fut = dialog_sink
            .send_all(caller_ch_stream.select(dialog_stream))
            .map(|_| ());

        Handler { f: Box::new(fut) }
    }

    fn receiver<F>(
        message: Frame,
        sender_map: &Arc<Mutex<HashMap<usize, oneshot::Sender<Bytes>>>>,
        f: &mut F,
    ) -> Box<Future<Item = Option<Frame>, Error = io::Error> + Send + Sync>
    where
        F: FnMut(Bytes) -> Box<Future<Item = Bytes, Error = io::Error> + Send + Sync> + Send,
    {
        let (t, id, payload) = message.into();
        match t {
            TypeLabel::Request => {
                let send_fut =
                    f(payload).map(move |resp| Some(Frame::new(TypeLabel::Response, id, resp)));
                Box::new(send_fut)
            }
            TypeLabel::Response => {
                if let Some(mut c) = sender_map.lock().remove(&(id as usize)) {
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
    fn poll(&mut self, cx: &mut task::Context) -> Poll<Self::Item, Self::Error> {
        self.f.poll(cx)
    }
}
