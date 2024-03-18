// #![feature(future_join)]
// use std::future::join;


// fn main() {
//     println!("Hello, world!");
//     join!(async{1},async{2});
// }
#![feature(prelude_import)]
#![feature(future_join)]
#[prelude_import]
use std::prelude::rust_2021::*;
#[macro_use]
extern crate std;
use std::{future::{join, poll_fn}, pin::Pin, task::Poll};
fn main() {
    {
        ::std::io::_print(format_args!("Hello, world!\n"));
    };
    match (MaybeDone::Future(async { 1 }), MaybeDone::Future(async { 2 })) {
        futures => {
            async {
                let mut futures = futures;
                let mut futures = unsafe { Pin::new_unchecked(&mut futures) };
                poll_fn(move |cx| {
                        let mut done = true;
                        let fut = unsafe {
                            futures
                                .as_mut()
                                .map_unchecked_mut(|it| {
                                    let (fut, ..) = it;
                                    fut
                                })
                        };
                        done &= fut.poll(cx).is_ready();
                        let fut = unsafe {
                            futures
                                .as_mut()
                                .map_unchecked_mut(|it| {
                                    let (_, fut, ..) = it;
                                    fut
                                })
                        };
                        done &= fut.poll(cx).is_ready();
                        if !done {
                            return Poll::Pending;
                        }
                        let futures = unsafe { futures.as_mut().get_unchecked_mut() };
                        Poll::Ready((
                            {
                                let (fut, ..) = &mut *futures;
                                fut.take_output().unwrap()
                            },
                            {
                                let (_, fut, ..) = &mut *futures;
                                fut.take_output().unwrap()
                            },
                        ))
                    })
                    .await
            }
        }
    };
}