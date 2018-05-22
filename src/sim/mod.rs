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
use std::io;
use std::sync::Arc;

struct Sim(Arc<Handler>);

impl Sim {
    fn new(h: Handler) -> Self {
        Sim(Arc::new(h))
    }

    #[allow(dead_code)]
    fn add<A: AsyncRead + AsyncWrite + Send + Sync + 'static>(
        &self,
        io: A,
    ) -> (Requestor, impl Future<Item = (), Error = io::Error>) {
        let handler = Arc::clone(&self.0);
        let (caller, handler) = io.dialog(move |caller, request| {
            let fut = match Request::from_bytes(request) {
                Some(Request {
                    kind: RequestType::Rpc,
                    topic,
                    message,
                }) => {
                    let handler = &*handler;
                    Self::rpc_handler(caller, handler, topic, message)
                        as Box<Future<Item = _, Error = _> + Send + Sync>
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
        handler: &Handler,
        topic: Bytes,
        message: Bytes,
    ) -> Box<Future<Item = Response, Error = io::Error> + Send + Sync> {
        match handler.get_rpc(&topic) {
            Some(call_handler) => Box::new(
                call_handler(Requestor::new(caller), message)
                    .map(|x| RpcResponse::Accepted(x).into()),
            ) as Box<Future<Item = _, Error = _> + Send + Sync>,
            None => Box::new(ok(RpcResponse::TopicNotFound.into()))
                as Box<Future<Item = _, Error = _> + Send + Sync>,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use futures::executor::{spawn, block_on};
    use util::PairIO;

    fn is_sync<T: Sync>() {}
    fn is_send<T: Send>() {}

    #[test]
    fn test_bounds() {
        is_send::<Handler>();
        is_send::<Requestor>();
        is_sync::<Handler>();
        is_sync::<Requestor>();
        is_send::<Sim>();
        is_sync::<Sim>();
    }

    #[test]
    fn simple_rpc() {
        let mut handler = Handler::new();
        let topic_echo = BytesMut::from(r"echo").freeze();
        let topic_del = BytesMut::from(r"del").freeze();
        handler.on_rpc(topic_echo.clone(), Box::new(|_, req| Box::new(ok(req))));
        handler.on_rpc(
            topic_del.clone(),
            Box::new(|_, _| Box::new(ok(BytesMut::from(&[] as &[u8]).freeze()))),
        );
        let sim = Sim::new(handler);

        let (io1, io2) = PairIO::new();
        let (mut req1, fut1) = sim.add(io1);
        let (_req2, fut2) = sim.add(io2);

        block_on(spawn(fut1.map_err(|e| panic!("fut1 panic {:?}", e)))).unwrap();
        block_on(spawn(fut2.map_err(|e| panic!("fut2 panic {:?}", e)))).unwrap();

        let hello = BytesMut::from(r"hello").freeze();
        let f1 = req1.rpc(topic_del, hello.clone()).inspect(|(_, resp)| {
            assert_eq!(
                resp,
                &RpcResponse::Accepted(BytesMut::from(&[] as &[u8]).freeze())
            )
        });
        let f2 = req1
            .rpc(BytesMut::from(r"new").freeze(), hello.clone())
            .inspect(|(_, resp)| assert_eq!(resp, &RpcResponse::TopicNotFound));
        let f3 = req1
            .rpc(topic_echo, hello.clone())
            .inspect(|(_, resp)| assert_eq!(resp, &RpcResponse::Accepted(hello.clone())));
        let _ = block_on(f1.join(f2).join(f3)).unwrap();
    }
}
