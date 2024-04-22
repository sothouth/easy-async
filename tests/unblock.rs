use std::io::{Cursor, SeekFrom};
use std::sync::mpsc;
use std::thread;

use futures::prelude::*;

use easy_async::block_on;
use easy_async::Unblock;

#[test]
fn chan() {
    const N: i32 = 100_000;

    block_on(async {
        let (s, r) = mpsc::sync_channel::<i32>(100);
        let handle = thread::spawn(move || {
            for i in 0..N {
                s.send(i).unwrap();
            }
        });

        let mut r = Unblock::new(r.into_iter());
        for i in 0..N {
            assert_eq!(r.next().await, Some(i));
        }

        handle.join().unwrap();
        assert!(r.next().await.is_none());
    })
}

#[test]
fn read() {
    const N: usize = 20_000_000;

    block_on(async {
        let mut v1 = vec![0u8; N];
        for i in 0..v1.len() {
            v1[i] = i as u8;
        }
        let mut v1 = Unblock::new(Cursor::new(v1));

        let mut v2 = vec![];
        v1.read_to_end(&mut v2).await.unwrap();

        let v1 = v1.into_inner().await.into_inner();
        assert!(v1 == v2);
    })
}

#[test]
fn write() {
    const N: usize = 20_000_000;

    block_on(async {
        let mut v1 = vec![0u8; N];
        for i in 0..v1.len() {
            v1[i] = i as u8;
        }

        let v2 = vec![];
        let mut v2 = Unblock::new(Cursor::new(v2));
        v2.write_all(&v1).await.unwrap();

        let v2 = v2.into_inner().await.into_inner();
        assert!(v1 == v2);
    })
}

#[test]
fn seek() {
    block_on(async {
        let len = 1_000;
        let mut v = vec![0u8; len];
        for i in 0..len {
            v[i] = i as u8;
        }
        let mut v = Unblock::new(Cursor::new(v));

        assert_eq!(v.seek(SeekFrom::Current(7i64)).await.unwrap(), 7);
        assert_eq!(v.seek(SeekFrom::Current(8i64)).await.unwrap(), 15);

        let mut byte = [0u8];
        v.read(&mut byte).await.unwrap();
        assert_eq!(byte[0], 15);
    })
}
