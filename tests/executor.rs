#[test]
fn spawn() {
    use easy_async::executor::executor::spawn;

    easy_async::block_on(spawn(async {
        println!("hello");
    }));
}
