#![feature(future_join)]
use std::future::join;
use std::net::{TcpListener, TcpStream};

use futures::StreamExt;
use futures::prelude::*;
use futures_core::Stream;
// use tokio::io::{AsyncReadExt, AsyncWriteExt};
// use tokio::net::{TcpListener, TcpStream};

use easy_async::block_on;
use easy_async::spawn;
use easy_async::Unblock;

fn main() {
    let fut = join!(serve(), client(), client());
    block_on(spawn(fut));
}

async fn serve() {
    println!("Server Start");
    let listener = Unblock::new(TcpListener::bind("127.0.0.1:29999").unwrap());
    for _ in 0..2 {
        let ret = listener.poll_read().await;
        if let Ok((socket, addr)) = ret {
            tokio::spawn(async move {
                handle_connection(socket).await;
            });
        }
    }
    println!("Server Done");
}

async fn handle_connection(mut socket: TcpStream) {
    println!("Connection Start");
    let (mut reader, mut writer) = socket.split();
    let mut buf = [0; 1024];
    for i in 0..10 {
        let n = reader.read(&mut buf).await.unwrap();
        if n == 0 {
            break;
        }
        let num = std::str::from_utf8(&buf[..n])
            .unwrap()
            .parse::<i32>()
            .unwrap();
        if num != i {
            println!("Error {}!={}", i, num);
        }
        writer.write_all(&buf[..n]).await.unwrap();
        writer.flush().await.unwrap();
    }
    println!("Connection Done");
}

async fn client() {
    println!("Client Start");
    let mut socket = TcpStream::connect("127.0.0.1:29999").await.unwrap();
    let (mut reader, mut writer) = socket.split();
    let mut buf = [0; 1024];
    for i in 0..10 {
        let msg = format!("{}", i);
        writer.write_all(msg.as_bytes()).await.unwrap();
        writer.flush().await.unwrap();

        let n = reader.read(&mut buf).await.unwrap();
        let num = std::str::from_utf8(&buf[..n])
            .unwrap()
            .parse::<i32>()
            .unwrap();
        if num != i {
            println!("Error {}!={}", i, num);
        }
    }
    println!("Client Done");
}
