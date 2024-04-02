use easy_async::executor::block_on::block_on;
use easy_async::utils::pending_n::PendingN;

#[test]
fn try_block_on() {
    assert_eq!(block_on(PendingN::new(10)), ());
}
