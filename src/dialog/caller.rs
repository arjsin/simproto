use std::io;
use std::sync::atomic::{AtomicUsize, ATOMIC_USIZE_INIT, Ordering};
use std::sync::Arc;
use std::hash::{Hash, Hasher};

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

    pub fn call(self, request: Bytes) -> Box<Future<Item = (Caller, Bytes), Error = io::Error>> {
        let (tx, rx) = oneshot::channel::<Bytes>();
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let Self {handler_ch, next_id} = self;
        let handler_ch_fut = handler_ch.send((id, tx, request));
        Box::new(
            handler_ch_fut.map_err(|_| io::Error::new(io::ErrorKind::Other, "send failed from caller"))
            .and_then(|handler_ch| rx.map(|resp|(resp, handler_ch)).map_err(|_| panic!("oneshot tx dropped"))
                .map(|(resp, handler_ch)| (Caller{handler_ch, next_id}, resp)),
            )
        )
    }
}

impl PartialEq for Caller {
    fn eq(&self, other: &Caller) -> bool {
        Arc::ptr_eq(&self.next_id, &other.next_id)
    }
}

impl Eq for Caller {}

impl Hash for Caller {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (&*self.next_id as *const AtomicUsize).hash(state);
    }
}
