use super::message::{Request, RequestType};
use bytes::{Bytes, BytesMut};
use dialog::Caller;
use futures::channel::mpsc;
use futures::prelude::*;
use futures::stream;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::collections::HashSet;
use std::io;
use std::sync::Arc;

type RequestHandler =
    Box<Fn(Bytes) -> Box<Future<Item = Bytes, Error = io::Error> + Send + Sync> + Send + Sync>;

pub struct Handler {
    call_handler: HashMap<Bytes, RequestHandler>,
    sub_handler: HashMap<Bytes, (RequestHandler, RequestHandler)>,
    notify_map: HashMap<Bytes, Arc<RwLock<HashSet<Caller>>>>,
}

impl Handler {
    pub fn new() -> Handler {
        Handler {
            call_handler: HashMap::new(),
            sub_handler: HashMap::new(),
            notify_map: HashMap::new(),
        }
    }

    pub fn on_rpc(&mut self, topic: Bytes, handler: RequestHandler) {
        self.call_handler.insert(topic, handler);
    }

    pub fn on_subs(
        &mut self,
        topic: Bytes,
        sub_handler: RequestHandler,
        unsub_handler: RequestHandler,
    ) -> (
        impl Sink<SinkItem = Bytes, SinkError = mpsc::SendError> + Send + Sync,
        impl Future<Item = (), Error = Never> + Send + Sync,
    ) {
        self.sub_handler
            .insert(topic.clone(), (sub_handler, unsub_handler));
        let (sink, stream) = mpsc::channel(1);
        let callers = Arc::default();
        self.notify_map.insert(topic.clone(), Arc::clone(&callers));
        let fut = stream
            .for_each(move |notification: Bytes| {
                let mut notify_request = BytesMut::new();
                Request::new(RequestType::Notification, topic.clone(), notification)
                    .write(&mut notify_request);
                let notify_request = notify_request.freeze();
                let callers = callers.read().clone();
                stream::iter_ok(callers.into_iter())
                    .for_each_concurrent(move |x| {
                        x.call(notify_request.clone()).map(|_| ()).recover(|_| ())
                    })
                    .map(|_| ())
            })
            .map(|_| ());
        (sink, fut)
    }

    pub fn add_notify_caller(&self, topic: &Bytes, caller: Caller) -> Option<bool> {
        let val = self.notify_map.get(topic)?.write().insert(caller);
        Some(val)
    }

    pub fn get_rpc(&self, topic: &Bytes) -> Option<&RequestHandler> {
        self.call_handler.get(topic)
    }

    pub fn get_subs(&self, topic: &Bytes) -> Option<&RequestHandler> {
        Some(&self.sub_handler.get(topic)?.0)
    }

    pub fn get_unsubs(&self, topic: &Bytes) -> Option<&RequestHandler> {
        Some(&self.sub_handler.get(topic)?.1)
    }
}
