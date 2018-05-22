use std::io;
use std::sync::atomic::{AtomicUsize, ATOMIC_USIZE_INIT, Ordering};
use std::sync::Arc;

use bytes::Bytes;
use futures::channel::{mpsc, oneshot};
use futures::prelude::*;

#[derive(Clone, Debug)]
pub struct Caller {
    handler_ch: mpsc::Sender<(usize, oneshot::Sender<Bytes>, Bytes)>,
    next_id: Arc<AtomicUsize>,
}

impl Caller {
    pub fn new(handler_ch: mpsc::Sender<(usize, oneshot::Sender<Bytes>, Bytes)>) -> Caller {
        Caller {
            handler_ch,
            next_id: Arc::new(ATOMIC_USIZE_INIT),
        }
    }

    pub fn call(&self, request: Bytes) -> Box<Future<Item = (Caller, Bytes), Error = io::Error>> {
        let (tx, rx) = oneshot::channel::<Bytes>();
        let next_id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let handler_ch = self.handler_ch.clone();
        let handler_ch_fut = handler_ch.send((next_id, tx, request));
        let next_id = Arc::clone(&self.next_id);
        Box::new(
            handler_ch_fut.map_err(|e| io::Error::new(io::ErrorKind::Other, "send failed from dialog::caller"))
            .and_then(|handler_ch| rx.map(|resp|(resp, handler_ch)).map_err(|_| panic!("oneshot tx dropped"))
                .map(|(resp, handler_ch)| (Caller{handler_ch, next_id}, resp)),
            )
        )
    }
}
