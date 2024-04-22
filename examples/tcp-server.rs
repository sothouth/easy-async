// use std::io;
use std::net::{TcpListener, TcpStream};

use easy_async::block_on;
use easy_async::Unblock;

use smol::{io, Async};

async fn echo(stream: Async<TcpStream>) {
    io::copy(&stream, &mut &stream).await.unwrap();
}

fn main() -> io::Result<()> {
    block_on(async {
        // let listener= Unblock::new(TcpListener::bind("127.0.0.1:7000")?);
        // println!("Listening on {}", listener.get_ref().local_addr()?);
        // println!("Now start a TCP client.");
        // loop {
        //     let (stream, peer_addr) = listener.accept().await?;
        //     println!("Accepted client: {}", peer_addr);
        //     easy_async::spawn(echo(stream));
        // }
        let listener = Async::<TcpListener>::bind(([127, 0, 0, 1], 7000))?;
        println!("Listening on {}", listener.get_ref().local_addr()?);
        println!("Now start a TCP client.");
        loop {
            let (stream, peer_addr) = listener.accept().await?;
            println!("Accepted client: {}", peer_addr);
            easy_async::spawn(echo(stream));
        }
    })
}
