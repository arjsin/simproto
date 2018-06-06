use super::message::{Request, RequestType, Response, RpcResponse, SubscriptionResponse};
use bytes::{Bytes, BytesMut};
use dialog::Caller;
use futures::channel::mpsc;
use futures::prelude::*;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::io;
use std::sync::Arc;

#[derive(Clone)]
pub struct Requestor {
    caller: Caller,
    subs: Arc<RwLock<HashMap<Bytes, mpsc::Sender<Bytes>>>>,
}

impl Requestor {
    pub fn new(
        caller: Caller,
        subs: Arc<RwLock<HashMap<Bytes, mpsc::Sender<Bytes>>>>,
    ) -> Requestor {
        Requestor { caller, subs }
    }

    pub fn rpc(
        self,
        topic: Bytes,
        data: Bytes,
    ) -> impl Future<Item = (Requestor, RpcResponse), Error = io::Error> {
        let mut request = BytesMut::new();
        Request::new(RequestType::Rpc, topic, data).write(&mut request);
        let Requestor { caller, subs } = self;
        caller.call(request.freeze()).map(|(caller, response)| {
            let response = Response::from_bytes(response);
            (Self::new(caller, subs), response.into())
        })
    }

    pub fn sub(
        self,
        topic: Bytes,
        data: Bytes,
    ) -> impl Future<
        Item = (
            Requestor,
            SubscriptionResponse,
            Option<mpsc::Receiver<Bytes>>,
        ),
        Error = io::Error,
    > {
        let mut request = BytesMut::new();
        Request::new(RequestType::Subscription, topic.clone(), data).write(&mut request);
        let Requestor { caller, subs } = self;
        caller.call(request.freeze()).map(|(caller, response)| {
            let response = Response::from_bytes(response).into();
            let chan = if let SubscriptionResponse::Accepted(_) = response {
                let mut subs = subs.write();
                let (ch_sink, ch_stream) = mpsc::channel(1);
                subs.insert(topic, ch_sink);
                Some(ch_stream)
            } else {
                None
            };
            (Self::new(caller, subs), response, chan)
        })
    }
}
