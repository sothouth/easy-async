use futures::{channel::mpsc, StreamExt};

fn main() {
    const N: usize = 10;
    const R: usize = (N * (N + 1)) / 2;

    let (tx, mut rx) = mpsc::unbounded();

    let handle = easy_async::spawn(async move {
        let mut result = 0;
        while let Some(i) = rx.next().await {
            result += i;
        }
        result
    });

    for i in 0..=N {
        tx.unbounded_send(i).unwrap();
    }

    drop(tx);

    assert_eq!(easy_async::block_on(handle), R);
}
