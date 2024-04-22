use std::net::TcpStream;

use easy_async::block_on;
use easy_async::spawn_blocking;

fn main() {
    block_on(async {
        let _ = spawn_blocking(|| TcpStream::connect("127.0.0.1:7000")).await;
    });
}
