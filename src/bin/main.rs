#![feature(future_join)]


use std::{future::{join, poll_fn}, pin::Pin, task::Poll};
fn main() {
    println!("Hello, world!");
    join!(async{1},async{2});
}