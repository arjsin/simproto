mod handler;
mod message;
mod requestor;

pub use self::handler::Handler;
use self::message::{Request, RequestType};
use self::message::{Response, RpcResponse};
pub use self::requestor::Requestor;

use bytes::{Bytes, BytesMut};
use dialog::{Caller, Dialog};
use futures::future::ok;
use futures::io::{AsyncRead, AsyncWrite};
use futures::prelude::*;
use std::cell::RefCell;
use std::io;
use std::rc::Rc;

struct Sim(Rc<RefCell<Handler>>);

impl Sim {
    fn new(h: Handler) -> Self {
        Sim(Rc::new(RefCell::new(h)))
    }

    #[allow(dead_code)]
    fn add<A: AsyncRead + AsyncWrite + 'static>(
        &self,
        io: A,
    ) -> (Requestor, impl Future<Item = (), Error = io::Error>) {
        let handler = Rc::clone(&self.0);
        let (caller, handler) = io.dialog(move |caller, request| {
            let fut = match Request::from_bytes(request) {
                Some(Request {
                    kind: RequestType::Rpc,
                    topic,
                    message,
                }) => {
                    let mut handler = handler.borrow_mut();
                    Self::rpc_handler(caller, &mut handler, topic, message)
                        as Box<Future<Item = _, Error = _>>
                }
                Some(Request {
                    kind: RequestType::Subscription,
                    topic,
                    message,
                }) => Box::new(ok(Response::InvalidRequest)),
                Some(Request {
                    kind: RequestType::Unsubscription,
                    topic,
                    message,
                }) => Box::new(ok(Response::InvalidRequest)),
                Some(Request {
                    kind: RequestType::Notification,
                    topic,
                    message,
                }) => Box::new(ok(Response::InvalidRequest)),
                None => Box::new(ok(Response::InvalidRequest)),
            }.map(|resp| {
                let mut resp_message = BytesMut::new();
                resp.write(&mut resp_message);
                resp_message.freeze()
            });
            Box::new(fut)
        });
        // TODO: handler.and_then{ to remove from subs lists }
        (Requestor::new(caller), handler)
    }

    fn rpc_handler(
        caller: Caller,
        handler: &mut Handler,
        topic: Bytes,
        message: Bytes,
    ) -> Box<Future<Item = Response, Error = io::Error>> {
        match handler.get_rpc(&topic) {
            Some(call_handler) => Box::new(
                call_handler(Requestor::new(caller), message)
                    .map(|x| RpcResponse::Accepted(x).into()),
            ) as Box<Future<Item = _, Error = _>>,
            None => {
                Box::new(ok(RpcResponse::TopicNotFound.into())) as Box<Future<Item = _, Error = _>>
            }
        }
    }
}
