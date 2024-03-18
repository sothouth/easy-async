pub struct Join<const N: usize, T: Future> {
    futures: [T; N],
    idx: usize,
}

impl<const N: usize, T: Future> Future for Join<N, T> {
    type Output = ();
}

pub macro join($($fut:expr),+$(,)?) {}
