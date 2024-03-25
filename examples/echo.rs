#![feature(future_join)]
use futures::StreamExt;
use std::future::join;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::runtime::{Builder, Runtime};
use tokio::spawn;

fn main() {
    let rt = Builder::new_current_thread().enable_all().build().unwrap();
    // rt.block_on(serve());
    rt.block_on(join!(serve(), client(), client()));
}

async fn serve() {
    println!("Server Start");
    let listener = TcpListener::bind("127.0.0.1:29999").await.unwrap();
    for _ in 0..2 {
        let ret = listener.accept().await;
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
