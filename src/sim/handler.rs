use super::requestor::Requestor;
use bytes::Bytes;
use futures::prelude::*;
use std::collections::HashMap;
use std::io;

type RequestHandler = Box<
    Fn(Requestor, Bytes) -> Box<Future<Item = Bytes, Error = io::Error> + Send + Sync>
        + Send
        + Sync,
>;

pub struct Handler {
    call_handler: HashMap<Bytes, RequestHandler>,
    sub_handler: HashMap<Bytes, (RequestHandler, RequestHandler)>,
}

impl Handler {
    pub fn new() -> Handler {
        Handler {
            call_handler: HashMap::new(),
            sub_handler: HashMap::new(),
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
    ) {
        self.sub_handler.insert(topic, (sub_handler, unsub_handler));
    }

    pub fn get_rpc(&self, topic: &Bytes) -> Option<&RequestHandler> {
        self.call_handler.get(topic)
    }

    pub fn get_subs(&self, topic: &Bytes) -> Option<&RequestHandler> {
        self.call_handler.get(topic)
    }

    pub fn get_unsubs(&mut self, topic: &Bytes) -> Option<&RequestHandler> {
        self.call_handler.get(topic)
    }
}
