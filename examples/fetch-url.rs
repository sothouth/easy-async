use std::net::TcpStream;

use easy_async::block_on;
use easy_async::spawn_blocking;

fn main() {
    easy_async::block_on(async {
        let mut stream = spawn_blocking(|| TcpStream::connect("127.0.0.1:7000"))
            .await
            .unwrap();

        
    });
}
