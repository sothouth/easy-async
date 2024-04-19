use std::alloc;
use std::alloc::Layout;
use std::future::Future;
use std::marker::PhantomData;
use std::mem;
use std::mem::ManuallyDrop;
use std::pin::Pin;
use std::ptr;
use std::ptr::NonNull;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::*;
use std::task::{Context, Poll};
// use std::task::{RawWaker, RawWakerVTable, Waker};

// use crossbeam_utils::CachePadded;

use crate::waker::AtomicWaker;

pub fn task_and_handle<F, T>(func: F) -> (OnceTask, OnceTaskHandle<T>)
where
    F: FnOnce() -> T,
{
    let ptr = RawOnceTask::<F, T>::allocate(func);

    let task = OnceTask::from_raw(ptr);
    let handle = OnceTaskHandle::from_raw(ptr);

    (task, handle)
}

// Task's state

/// Task is scheduled.
const SCHEDULED: usize = 1 << 0;
/// Task is running.
const RUNNING: usize = 1 << 1;
/// Task is completed.
///
/// The future is end.
const COMPLETED: usize = 1 << 2;
/// Task is closed.
///
/// The future is end and the output is taken.
const CLOSED: usize = 1 << 3;

struct Header {
    state: AtomicUsize,
    refer: AtomicUsize,
    waker: AtomicWaker,
    vtable: &'static OnceTaskVTable,
}

impl Header {
    fn new(vtable: &'static OnceTaskVTable) -> Self {
        Self {
            state: AtomicUsize::new(SCHEDULED),
            refer: AtomicUsize::new(0),
            waker: AtomicWaker::new(),
            vtable,
        }
    }

    #[inline]
    fn decrement_refer(&self, ptr: *const ()) {
        if self.refer.fetch_sub(1, AcqRel) == 1 {
            unsafe { (self.vtable.destroy)(ptr) }
        }
    }

    #[inline]
    fn increment_refer(&self) {
        self.refer.fetch_add(1, AcqRel);
    }
}

struct OnceTaskLayout {
    layout: Layout,
    offset_data: usize,
}

impl OnceTaskLayout {
    const fn new<F, T>() -> Self {
        let header = Layout::new::<Header>();
        let data = Layout::new::<Data<F, T>>();

        let (layout, offset_data) = Self::extend(header, data);

        Self {
            layout,
            offset_data,
        }
    }

    const fn extend(pre: Layout, nex: Layout) -> (Layout, usize) {
        let align = Self::max(pre.align(), nex.align());
        let pad = pre.padding_needed_for(nex.align());

        let Some(offset) = pre.size().checked_add(pad) else {
            panic!("OnceTaskLayout extend offset overflow");
        };
        let Some(size) = offset.checked_add(nex.size()) else {
            panic!("OnceTaskLayout extend size overflow");
        };

        let Ok(layout) = Layout::from_size_align(size, align) else {
            panic!("OnceTaskLayout construct layout error");
        };

        (layout, offset)
    }

    const fn max(a: usize, b: usize) -> usize {
        if a > b {
            a
        } else {
            b
        }
    }
}

struct OnceTaskVTable {
    drop_fn: unsafe fn(*const ()),
    get_output: unsafe fn(*const ()) -> *const (),
    destroy: unsafe fn(*const ()),
    run: unsafe fn(*const ()),
    layout: &'static OnceTaskLayout,
}

union Data<F, T> {
    func: ManuallyDrop<F>,
    output: ManuallyDrop<T>,
}

struct RawOnceTask<F, T> {
    header: *const Header,
    data: *mut Data<F, T>,
}

// Impl const `RawOnceTask`
impl<F, T> RawOnceTask<F, T> {
    const LAYOUT: OnceTaskLayout = OnceTaskLayout::new::<F, T>();

    #[inline]
    const fn from_ptr(ptr: *const ()) -> Self {
        let ptr = ptr as *const u8;

        unsafe {
            Self {
                header: ptr as *const Header,
                data: ptr.add(Self::LAYOUT.offset_data) as *mut Data<F, T>,
            }
        }
    }
}

// Impl `TaskVTable` for `RawOnceTask`
impl<F, T> RawOnceTask<F, T>
where
    F: FnOnce() -> T,
{
    unsafe fn drop_fn(ptr: *const ()) {
        let raw = Self::from_ptr(ptr);

        ManuallyDrop::drop(&mut (*raw.data).func);
    }

    unsafe fn get_output(ptr: *const ()) -> *const () {
        let raw = Self::from_ptr(ptr);

        (&*(*raw.data).output) as *const T as *const ()
    }

    unsafe fn destroy(ptr: *const ()) {
        let raw = Self::from_ptr(ptr);
        let header = &*(raw.header as *const Header);

        match header.state.load(Acquire) {
            SCHEDULED => {
                (header.vtable.drop_fn)(ptr);
            }
            COMPLETED => {
                let _ = (header.vtable.get_output)(ptr);
            }
            _ => {}
        }

        (raw.header as *mut Header).drop_in_place();
        (raw.data as *mut Data<F, T>).drop_in_place();

        alloc::dealloc(ptr as *mut u8, Self::LAYOUT.layout);
    }

    unsafe fn run(ptr: *const ()) {
        let raw = Self::from_ptr(ptr);

        match (*raw.header)
            .state
            .compare_exchange(SCHEDULED, RUNNING, AcqRel, Acquire)
        {
            Ok(_) => {
                let output = ManuallyDrop::take(&mut (*raw.data).func)();
                (*raw.data).output = ManuallyDrop::new(output);

                (*raw.header).state.store(COMPLETED, Release);
                (*raw.header).waker.wake();
            }
            Err(cur) => {
                unreachable!("invalid run task state: {}", cur);
            }
        }
    }
}

impl<F, T> RawOnceTask<F, T>
where
    F: FnOnce() -> T,
{
    fn allocate(func: F) -> NonNull<()> {
        unsafe {
            let ptr = match NonNull::new(alloc::alloc(Self::LAYOUT.layout) as *mut ()) {
                Some(ptr) => ptr,
                None => alloc::handle_alloc_error(Self::LAYOUT.layout),
            };

            let raw = Self::from_ptr(ptr.as_ptr());

            (raw.header as *mut Header).write(Header::new(&OnceTaskVTable {
                drop_fn: Self::drop_fn,
                get_output: Self::get_output,
                destroy: Self::destroy,
                run: Self::run,
                layout: &Self::LAYOUT,
            }));

            raw.data.write(Data {
                func: ManuallyDrop::new(func),
            });

            ptr
        }
    }
}

pub struct OnceTask {
    ptr: NonNull<()>,
}

unsafe impl Send for OnceTask {}
unsafe impl Sync for OnceTask {}

impl OnceTask {
    fn from_raw(ptr: NonNull<()>) -> Self {
        unsafe { (*(ptr.as_ptr() as *const Header)).increment_refer() };

        Self { ptr }
    }

    pub fn run(self) {
        let ptr = self.ptr.as_ptr();
        let header = ptr as *const Header;
        mem::forget(self);

        unsafe { ((*header).vtable.run)(ptr) };
    }
}

impl Drop for OnceTask {
    fn drop(&mut self) {
        let ptr = self.ptr.as_ptr();

        unsafe { (*(ptr as *const Header)).decrement_refer(ptr) };
    }
}

pub struct OnceTaskHandle<T> {
    ptr: NonNull<()>,
    _marker: PhantomData<T>,
}

unsafe impl<T: Send> Send for OnceTaskHandle<T> {}
unsafe impl<T: Sync> Sync for OnceTaskHandle<T> {}

impl<T> Unpin for OnceTaskHandle<T> {}

impl<T> OnceTaskHandle<T> {
    fn from_raw(ptr: NonNull<()>) -> Self {
        unsafe { (*(ptr.as_ptr() as *const Header)).increment_refer() };

        Self {
            ptr,
            _marker: PhantomData,
        }
    }

    fn poll_task(&mut self, cx: &mut Context<'_>) -> Poll<Option<T>> {
        let ptr = self.ptr.as_ptr();
        let header = ptr as *const Header;

        match unsafe {
            (*header)
                .state
                .compare_exchange(COMPLETED, CLOSED, AcqRel, Acquire)
        } {
            Ok(_) => {
                return Poll::Ready(Some(unsafe {
                    ptr::read(((*header).vtable.get_output)(ptr) as *const T)
                }));
            }
            Err(state) => {
                if state & CLOSED != 0 {
                    return Poll::Ready(None);
                }

                unsafe { (*header).waker.register(cx.waker()) };

                return Poll::Pending;
            }
        }
    }
}

impl<T> Drop for OnceTaskHandle<T> {
    fn drop(&mut self) {
        let ptr = self.ptr.as_ptr();

        unsafe { (*(ptr as *const Header)).decrement_refer(ptr) };
    }
}

impl<T> Future for OnceTaskHandle<T> {
    type Output = T;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.poll_task(cx) {
            Poll::Ready(t) => Poll::Ready(t.expect("task is closed")),
            Poll::Pending => Poll::Pending,
        }
    }
}
