use std::alloc::Layout;
use std::future::Future;
use std::marker::PhantomData;
use std::mem::ManuallyDrop;
use std::pin::Pin;
use std::ptr;
use std::ptr::NonNull;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::*;
use std::task::{Context, Poll};
use std::task::{RawWaker, RawWakerVTable, Waker};

use std::sync::Arc;

use crossbeam_utils::CachePadded;

use crate::waker::AtomicWaker;

// Task's state

/// Task is scheduled.
const SCHEDULED: usize = 1 << 0;
/// Task is running.
const RUNNING: usize = 1 << 1;
/// Task is completed.
///
/// The future is end.
const COMPLETED: usize = 1 << 3;
/// Task is closed.
///
/// The future is end and the output is taken.
const CLOSED: usize = 1 << 4;

// Task's reference count

/// Task's reference count one.
const REFERENCE: usize = 1 << 5;

struct Header {
    state: CachePadded<AtomicUsize>,
    refer: CachePadded<AtomicUsize>,
    waker: CachePadded<AtomicWaker>,
}

struct RawOnceTask<F, T> {
    header: *const Header,
    func: *mut F,
    output: *const T,
}

struct OnceTask {}

struct OnceTaskHandle<T> {
    ptr: NonNull<()>,
    _marker: PhantomData<T>,
}
