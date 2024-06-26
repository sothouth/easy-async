use std::alloc::{self, Layout};
use std::future::Future;
use std::marker::PhantomData;
use std::mem::{self, ManuallyDrop};
use std::pin::Pin;
use std::ptr::{self, NonNull};
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::*;
use std::task::{Context, Poll};

use crate::waker::AtomicWaker;

/// Creates a [`OnceTask`] and a corresponding [`OnceTaskHandle`] for a given closure.
///
/// The function takes a closure `func` that is to be executed.
/// It returns a tuple containing a [`OnceTask`], which represents the executable task, and
/// a [`OnceTaskHandle`], which can be used to poll the task and retrieve its output.
///
/// # Examples
///
/// ```ignore
/// let (task, handle) = task_and_handle(|| {
///     // Perform some work...
///     42 // Return a value
/// });
/// ```
pub(crate) fn task_and_handle<F, T>(func: F) -> (OnceTask, OnceTaskHandle<T>)
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
/// This flag is set when the function has been runned.
const COMPLETED: usize = 1 << 2;
/// Task is closed.
///
/// This flag is set when the function has ended and its output has been consumed.
const CLOSED: usize = 1 << 3;

/// The [`Header`] struct contains metadata for the once-task.
///
/// It includes the current state of the task, a reference counter,
/// an [`AtomicWaker`] for waking up the task,
/// and a reference to the [`OnceTaskVTable`] for
/// dynamic dispatch of functions related to task management.
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

/// The [`OnceTaskLayout`] struct defines the memory layout for a once-task.
///
/// It includes the combined [`Layout`] of the [`Header`] and the [`Data`] and
/// the offset at which the [`Data`] starts within the layout.
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

/// The [`OnceTaskVTable`] struct contains function pointers for operations on [`OnceTask`].
///
/// It includes functions for retrieving the output of a completed task,
/// destroying the task, and running the task.
struct OnceTaskVTable {
    get_output: unsafe fn(*const ()) -> *const (),
    destroy: unsafe fn(*const ()),
    run: unsafe fn(*const ()),
}

/// The [`Data`] union holds the closure to be executed or the output of the closure.
union Data<F, T> {
    func: ManuallyDrop<F>,
    output: ManuallyDrop<T>,
}

/// The [`RawOnceTask`] struct is a low-level representation of a once-executable task.
///
/// It contains pointers to the [`Header`] and [`Data`] of the task.
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

        Self {
            header: ptr as *const Header,
            data: unsafe { ptr.add(Self::LAYOUT.offset_data) } as *mut Data<F, T>,
        }
    }
}

// Impl `OnceTaskVTable` for `RawOnceTask`
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
        let header = &*raw.header;

        match header.state.load(Acquire) {
            SCHEDULED => Self::drop_fn(ptr),
            COMPLETED => ManuallyDrop::drop(&mut (*raw.data).output),
            _ => {}
        }

        (raw.header as *mut Header).drop_in_place();
        raw.data.drop_in_place();

        alloc::dealloc(ptr as *mut u8, Self::LAYOUT.layout);
    }

    unsafe fn run(ptr: *const ()) {
        let raw = Self::from_ptr(ptr);
        let header = &*raw.header;

        match header
            .state
            .compare_exchange(SCHEDULED, RUNNING, AcqRel, Acquire)
        {
            Ok(_) => {
                let output = ManuallyDrop::take(&mut (*raw.data).func)();
                (*raw.data).output = ManuallyDrop::new(output);

                header.state.store(COMPLETED, Release);
                header.waker.wake();
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
    /// Allocates a new once-task based on [`OnceTaskLayout`] and the given closure.
    ///
    /// Return a pointer to the once-task.
    fn allocate(func: F) -> NonNull<()> {
        unsafe {
            let ptr = match NonNull::new(alloc::alloc(Self::LAYOUT.layout) as *mut ()) {
                Some(ptr) => ptr,
                None => alloc::handle_alloc_error(Self::LAYOUT.layout),
            };

            let raw = Self::from_ptr(ptr.as_ptr());

            (raw.header as *mut Header).write(Header::new(&OnceTaskVTable {
                get_output: Self::get_output,
                destroy: Self::destroy,
                run: Self::run,
                // layout: &Self::LAYOUT,
            }));

            raw.data.write(Data {
                func: ManuallyDrop::new(func),
            });

            ptr
        }
    }
}

/// The [`OnceTask`] struct represents a task.
///
/// It is used by [`crate::blocking`].
pub(crate) struct OnceTask {
    ptr: NonNull<()>,
}

unsafe impl Send for OnceTask {}
unsafe impl Sync for OnceTask {}

impl OnceTask {
    fn from_raw(ptr: NonNull<()>) -> Self {
        unsafe { (*(ptr.as_ptr() as *const Header)).increment_refer() };

        Self { ptr }
    }

    /// Runs the function.
    pub(crate) fn run(self) {
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

/// The [`OnceTaskHandle`] struct is a handle to a [`OnceTask`].
///
/// It implements [`Future`] and can be used to poll the task and retrieve its output.
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
        let header = unsafe { &*(ptr as *const Header) };

        match header
            .state
            .compare_exchange(COMPLETED, CLOSED, AcqRel, Acquire)
        {
            Ok(_) => Poll::Ready(Some(unsafe {
                ptr::read((header.vtable.get_output)(ptr) as *const T)
            })),
            Err(state) => {
                if state & CLOSED != 0 {
                    return Poll::Ready(None);
                }

                header.waker.register(cx.waker());

                Poll::Pending
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
