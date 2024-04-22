async fn fib(n: usize) -> usize {
    if n < 2 {
        return 1;
    }

    Box::pin(fib(n - 1)).await + Box::pin(fib(n - 2)).await
}

fn main() {
    println!("{}", easy_async::block_on(fib(30)));
}
