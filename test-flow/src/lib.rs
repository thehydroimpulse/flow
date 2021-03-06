#![feature(plugin)]
#![plugin(flow)]

extern crate tangle;

use tangle::{Future, Async};

#[test]
fn compile() {
    let foobar = Future::unit(123);
    let ccc = 123;
    flow!{
        let a: bool<-foobar(ccc)
        a
    };
}
