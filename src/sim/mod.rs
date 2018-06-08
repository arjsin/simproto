mod handler;
mod message;
mod requestor;

pub use self::handler::Handler;
pub use self::message::{
    NotificationResponse, Response, RpcResponse, SubscriptionResponse, UnsubscriptionResponse,
};
use self::message::{Request, RequestType};
pub use self::requestor::Requestor;

use bytes::{Bytes, BytesMut};
use crossbeam::sync::AtomicOption;
use dialog::{Caller, Dialog};
use futures::channel::mpsc::Sender;
use futures::future::ok;
use futures::io::{AsyncRead, AsyncWrite};
use futures::prelude::*;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::io;
use std::sync::atomic::Ordering;
use std::sync::Arc;

pub struct Sim(Arc<Handler>);

impl Sim {
    pub fn new(h: Handler) -> Self {
        Sim(Arc::new(h))
    }

    #[allow(dead_code)]
    pub fn add<A: AsyncRead + AsyncWrite + Send + Sync + 'static>(
        &self,
        io: A,
    ) -> (Requestor, impl Future<Item = (), Error = io::Error>) {
        let receiving_subs_map = Arc::new(RwLock::new(HashMap::new()));
        let caller_opt = Arc::new(AtomicOption::new()); // FIXME: this may not make inner Sync
        let (caller, handler) = {
            let subs_map = receiving_subs_map.clone();
            let handler = Arc::clone(&self.0);
            let caller_opt = caller_opt.clone();
            io.dialog(move |request| {
                let fut = match Request::from_bytes(request) {
                    Some(Request {
                        kind,
                        topic,
                        message,
                    }) => match kind {
                        RequestType::Rpc => Self::rpc_handler(&*handler, topic, message)
                            as Box<Future<Item = _, Error = _> + Send + Sync>,
                        RequestType::Subscription => {
                            let caller = caller_opt.take(Ordering::Relaxed).unwrap();
                            Self::sub_handler(handler.clone(), topic, message, caller)
                        }
                        RequestType::Unsubscription => {
                            Self::unsub_handler(&*handler, topic, message)
                        }
                        RequestType::Notification => {
                            Self::notify_handler(&subs_map, topic, message)
                        }
                    },
                    None => Box::new(ok(Response::InvalidRequest)),
                }.map(|resp| {
                    let mut resp_message = BytesMut::new();
                    resp.write(&mut resp_message);
                    resp_message.freeze()
                });
                Box::new(fut)
            })
        };
        // TODO: handler.and_then{ to remove from subs lists }
        caller_opt.swap(caller.clone(), Ordering::Relaxed);
        (Requestor::new(caller, receiving_subs_map), handler)
    }

    fn rpc_handler(
        handler: &Handler,
        topic: Bytes,
        message: Bytes,
    ) -> Box<Future<Item = Response, Error = io::Error> + Send + Sync> {
        match handler.get_rpc(&topic) {
            Some(call_handler) => Box::new(
                call_handler(message).map(|x| RpcResponse::Accepted(x).into()),
            ) as Box<Future<Item = _, Error = _> + Send + Sync>,
            None => Box::new(ok(RpcResponse::TopicNotFound.into())),
        }
    }

    fn sub_handler(
        handler: Arc<Handler>,
        topic: Bytes,
        message: Bytes,
        caller: Caller,
    ) -> Box<Future<Item = Response, Error = io::Error> + Send + Sync> {
        match handler.clone().get_subs(&topic) {
            // TODO: add rejection type in the call_handler
            Some(call_handler) => Box::new(call_handler(message).map(move |x| {
                match handler.add_notify_caller(&topic, caller) {
                    Some(true) => SubscriptionResponse::Accepted(x).into(),
                    Some(false) => SubscriptionResponse::AlreadySubscribed.into(),
                    None => SubscriptionResponse::TopicNotFound.into(),
                }
            })) as Box<Future<Item = _, Error = _> + Send + Sync>,
            None => Box::new(ok(SubscriptionResponse::TopicNotFound.into())),
        }
    }

    //TODO
    fn unsub_handler(
        handler: &Handler,
        topic: Bytes,
        message: Bytes,
    ) -> Box<Future<Item = Response, Error = io::Error> + Send + Sync> {
        match handler.get_unsubs(&topic) {
            Some(call_handler) => Box::new(
                call_handler(message).map(|x| UnsubscriptionResponse::Accepted(x).into()),
            ) as Box<Future<Item = _, Error = _> + Send + Sync>,
            None => Box::new(ok(UnsubscriptionResponse::TopicNotFound.into())),
        }
    }

    fn notify_handler(
        subs_map: &Arc<RwLock<HashMap<Bytes, Sender<Bytes>>>>,
        topic: Bytes,
        message: Bytes,
    ) -> Box<Future<Item = Response, Error = io::Error> + Send + Sync> {
        let subs_map_read = subs_map.read();
        let sub = subs_map_read.get(&topic);
        match sub {
            Some(sub) => {
                let subs_map = subs_map.clone();
                Box::new(
                    sub.clone()
                        .send(message)
                        .map(|_| NotificationResponse::Notified.into())
                        .recover(move |_| {
                            let mut subs_map = subs_map.write();
                            subs_map.remove(&topic);
                            NotificationResponse::NotSubscribed.into()
                        }),
                ) as Box<Future<Item = _, Error = _> + Send + Sync>
            }
            None => Box::new(ok(NotificationResponse::TopicNotFound.into())),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use futures::executor::{block_on, spawn};
    use util::PairIO;

    fn is_sync<T: Sync>() {}
    fn is_send<T: Send>() {}

    #[test]
    fn test_bounds() {
        is_send::<Handler>();
        is_send::<Requestor>();
        is_send::<Sim>();
        is_sync::<Handler>();
        is_sync::<Requestor>();
        is_sync::<Sim>();
    }

    #[test]
    fn simple_rpc() {
        let mut handler = Handler::new();
        let topic_echo = BytesMut::from(r"echo").freeze();
        let topic_del = BytesMut::from(r"del").freeze();
        handler.on_rpc(topic_echo.clone(), Box::new(|req| Box::new(ok(req))));
        handler.on_rpc(
            topic_del.clone(),
            Box::new(|_| Box::new(ok(BytesMut::from(&[] as &[u8]).freeze()))),
        );
        let sim = Sim::new(handler);

        let (io1, io2) = PairIO::new();
        let (req1, fut) = sim.add(io1);
        block_on(spawn(fut.map_err(|e| panic!("io1 sim fut panic {:?}", e)))).unwrap();

        let (_req2, fut) = sim.add(io2);
        block_on(spawn(fut.map_err(|e| panic!("io1 sim fut panic {:?}", e)))).unwrap();

        let hello = BytesMut::from(r"hello").freeze();
        let f1 = req1
            .clone()
            .rpc(topic_del, hello.clone())
            .inspect(|(_, resp)| {
                assert_eq!(
                    resp,
                    &RpcResponse::Accepted(BytesMut::from(&[] as &[u8]).freeze())
                )
            });
        let f2 = req1
            .clone()
            .rpc(BytesMut::from(r"new").freeze(), hello.clone())
            .inspect(|(_, resp)| assert_eq!(resp, &RpcResponse::TopicNotFound));
        let f3 = req1
            .rpc(topic_echo, hello.clone())
            .inspect(|(_, resp)| assert_eq!(resp, &RpcResponse::Accepted(hello.clone())));
        let _ = block_on(f1.join(f2).join(f3)).unwrap();
    }

    #[test]
    fn simple_notify() {
        use futures::future::ok;
        let mut handler = Handler::new();
        let topic_once = BytesMut::from(r"once").freeze();

        let (once_sink, fut) = handler.on_subs(
            topic_once.clone(),
            Box::new(|req| Box::new(ok(req))),
            Box::new(|req| Box::new(ok(req))),
        );
        block_on(spawn(fut.map_err(|e| panic!("on_sub fut panic {:?}", e)))).unwrap();
        let sim = Sim::new(handler);

        let (io1, io2) = PairIO::new();
        let (req1, fut) = sim.add(io1);
        block_on(spawn(fut.map_err(|e| panic!("io1 sim fut panic {:?}", e)))).unwrap();

        let (_req2, fut) = sim.add(io2);
        block_on(spawn(fut.map_err(|e| panic!("io2 sim fut panic {:?}", e)))).unwrap();

        let fut = once_sink
            .send(BytesMut::from(b"Hello" as &[u8]).freeze())
            .map(|_| ())
            .map_err(|e| panic!("Sending notification panic {:?}", e));

        let fut = req1
            .sub(topic_once, BytesMut::from(&[] as &[u8]).freeze())
            .and_then(|(_, resp, receiver)| fut.map(|_| (resp, receiver)))
            .and_then(|(resp, receiver)| {
                assert_eq!(
                    resp,
                    SubscriptionResponse::Accepted(BytesMut::from(&[] as &[u8]).freeze())
                );
                ok(receiver.unwrap().map_err(Never::never_into))
            })
            .flatten_stream()
            .take(1)
            .inspect(|x| assert_eq!(x, &BytesMut::from(r"Hello").freeze()))
            .for_each(|_| ok(()))
            .map(|_| ());

        block_on(fut.map_err(|e| panic!("Subscribe and get notification panic {:?}", e)))
            .unwrap();
    }
}
