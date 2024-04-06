#[test]
fn spawn() {
    use easy_async::executor::spawn::spawn;

    easy_async::block_on(spawn(async {
        println!("hello");
    }));
}
