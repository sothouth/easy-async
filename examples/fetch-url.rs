use std::net::TcpStream;

fn main() {
    easy_async::block_on(async {
        let _ = easy_async::spawn_blocking(|| TcpStream::connect("127.0.0.1:7000")).await;
    });
}
