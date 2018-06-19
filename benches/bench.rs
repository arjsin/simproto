#![feature(test)]
extern crate bytes;
extern crate futures;
extern crate simproto;
extern crate test;

use bytes::BytesMut;
use futures::executor::{block_on, spawn};
use futures::future::ok;
use futures::prelude::*;
use simproto::sim::{Handler, Sim, SubscriptionResponse};
use simproto::util::PairIO;
use test::{black_box, Bencher};

#[bench]
fn rpc(b: &mut Bencher) {
    let mut handler = Handler::new();
    let topic_echo = BytesMut::from(r"echo").freeze();
    handler.on_rpc(topic_echo.clone(), Box::new(|req| Box::new(ok(req))));
    let sim = Sim::new(handler);

    let (io1, io2) = PairIO::new();
    let (req1, fut) = sim.add(io1);
    block_on(spawn(fut.map_err(|e| panic!("io1 sim fut panic {:?}", e)))).unwrap();

    let (_req2, fut) = sim.add(io2);
    block_on(spawn(fut.map_err(|e| panic!("io2 sim fut panic {:?}", e)))).unwrap();

    let hello = BytesMut::from(r"hello").freeze();
    let req1 = black_box(req1);
    b.iter(|| {
        let f = req1.clone().rpc(topic_echo.clone(), hello.clone());
        let _ = block_on(f).unwrap();
    });
}
