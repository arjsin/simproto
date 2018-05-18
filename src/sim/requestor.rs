use super::message::{Request, RequestType};
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

    fn call(
        &mut self,
        topic: Bytes,
        data: Bytes,
    ) -> impl Future<Item = (Requestor, Bytes), Error = io::Error> {
        let mut request = BytesMut::new();
        Request::new(RequestType::Rpc, topic, data).write(&mut request);
        self.caller
            .call(request.freeze())
            .map(|(caller, response)| (Self::new(caller), response))
    }
}
