use std::cell::Cell;
use std::io;
use std::rc::Rc;

use bytes::Bytes;
use futures::channel::{mpsc, oneshot};
use futures::prelude::*;

#[derive(Clone, Debug)]
pub struct Caller {
    handler_ch: mpsc::Sender<(u64, oneshot::Sender<Bytes>, Bytes)>,
    next_id: Rc<Cell<u64>>,
}

impl Caller {
    pub fn new(handler_ch: mpsc::Sender<(u64, oneshot::Sender<Bytes>, Bytes)>) -> Caller {
        Caller {
            handler_ch,
            next_id: Rc::new(Cell::new(0u64)),
        }
    }

    pub fn call(&self, request: Bytes) -> Box<Future<Item = (Bytes, Caller), Error = io::Error>> {
        let (tx, rx) = oneshot::channel::<Bytes>();
        let next_id = self.next_id.take();
        let handler_ch = self.handler_ch.clone();
        let handler_ch_fut = handler_ch.send((next_id, tx, request));
        self.next_id.set(next_id.wrapping_add(1));
        let next_id = Rc::clone(&self.next_id);
        Box::new(
            handler_ch_fut.map_err(|_| io::Error::new(io::ErrorKind::Other, "send failed"))
            .and_then(|handler_ch| rx.map(|resp|(resp, handler_ch)).map_err(|_| panic!("oneshot tx dropped"))
                .map(|(resp, handler_ch)| (resp, Caller{handler_ch, next_id})),
            )
        )
    }
}
