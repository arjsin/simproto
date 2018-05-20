use super::message::{Request, RequestType, Response, RpcResponse};
use bytes::{Bytes, BytesMut};
use dialog::Caller;
use futures::prelude::*;
use std::io;

pub struct Requestor {
    caller: Caller,
    // TODO: list of sub
}

impl Requestor {
    pub fn new(caller: Caller) -> Requestor {
        Requestor { caller }
    }

    pub fn rpc(
        &mut self,
        topic: Bytes,
        data: Bytes,
    ) -> impl Future<Item = (Requestor, RpcResponse), Error = io::Error> {
        let mut request = BytesMut::new();
        Request::new(RequestType::Rpc, topic, data).write(&mut request);
        self.caller
            .call(request.freeze())
            .map(|(caller, response)| {
                let response = Response::from_bytes(response);
                (Self::new(caller), response.into())
            })
    }
}
