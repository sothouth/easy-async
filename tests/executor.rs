#[test]
fn spawn() {
    use easy_async::executor::local_executor::spawn;

    easy_async::block_on(spawn(async {
        println!("hello");
    }));
}
