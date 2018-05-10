use bytes::Bytes;
use futures::unsync::{mpsc, oneshot};
use futures::{Future, Sink};
use std::cell::Cell;
use std::io;
use std::rc::Rc;

#[derive(Clone, Debug)]
pub struct Caller {
    // Caller
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

    #[allow(dead_code)]
    pub fn call(self, request: Bytes) -> Box<Future<Item = (Bytes, Caller), Error = io::Error>> {
        let (tx, rx) = oneshot::channel::<Bytes>();
        let next_id = self.next_id.take();
        let sender = self.clone();
        let handler_ch_fut = self.handler_ch.send((next_id, tx, request));
        self.next_id.set(next_id.wrapping_add(1));
        // TODO: send and then receive
        Box::new(
            rx.map_err(|_| io::Error::new(io::ErrorKind::Other, "oneshot tx dropped"))
                .join(
                    handler_ch_fut.map_err(|_| io::Error::new(io::ErrorKind::Other, "send failed")),
                )
                .map(|(rx, _)| (rx, sender)),
        )
    }
}
