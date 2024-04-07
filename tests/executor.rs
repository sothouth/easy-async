#[test]
fn spawn() {
    use easy_async::executor::executor::spawn;

    easy_async::block_on(spawn(async {
        println!("hello");
    }));
}

#[test]
fn smol_spawn() {
    // use smol::block_on;
    use easy_async::block_on;
    use smol::spawn;
    block_on(spawn(async {
        println!("hello");
    }));
}
