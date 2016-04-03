#![feature(plugin)]
#![plugin(flow)]

extern crate tangle;

use tangle::{Future, Async};

#[test]
fn compile() {
    flow!{
        let a: bool<-foobar
    };

    // foobar.and_then(move |a| {
    //     Async::Ok(())
    // })
}
