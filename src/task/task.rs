use std::alloc::{self, Layout};
use std::future::Future;
use std::marker::PhantomData;
use std::mem::{self, ManuallyDrop};
use std::pin::Pin;
use std::ptr::{self, NonNull};
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::*;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::task::{RawWaker, RawWakerVTable, Waker};

use crate::executor::Runtime;
use crate::waker::AtomicWaker;

/// Creates a [`Task`] and a corresponding [`TaskHandle`] for a given [`Future`].
///
/// The function takes a `future` that is to be executed asynchronously.
/// It returns a tuple containing a [`Task`], which represents the executable task, and
/// a [`TaskHandle`], which can be used to poll the task and retrieve its output.
///
/// # Examples
///
/// ```no_run
/// let (task, handle) = task_and_handle(async move {
///     // Perform some work...
///     42 // Return a value
/// });
/// ```
pub(crate) fn task_and_handle<F, T>(future: F, rt: &Arc<Runtime>) -> (Task, TaskHandle<T>)
where
    F: Future<Output = T> + Send + 'static,
{
    let ptr = RawTask::<F, T>::allocate(future, rt);

    let task = Task::from_raw(ptr);
    let handle = TaskHandle::from_raw(ptr);

    (task, handle)
}

// Task's state

/// Task is sleeping.
const SLEEPING: usize = 1 << 0;

/// Task is scheduled.
const SCHEDULED: usize = 1 << 1;
/// Task is running.
const RUNNING: usize = 1 << 2;
/// Task is completed.
///
/// This flag is set when the function has been runned.
const COMPLETED: usize = 1 << 3;
/// Task is closed.
///
/// This flag is set when the function has ended and its output has been consumed.
const CLOSED: usize = 1 << 4;

/// The [`Header`] struct contains metadata for the task.
///
/// It includes the current state of the task,
/// a reference counter, an [`AtomicWaker`] for waking up the task,
/// and a reference to the [`TaskVTable`] for
/// dynamic dispatch of functions related to task management.
///
/// And also contains the task will be scheduled queue id and the [`Runtime`].
struct Header {
    state: AtomicUsize,
    refer: AtomicUsize,
    waker: AtomicWaker,
    rt: Arc<Runtime>,
    queue_id: AtomicUsize,
    vtable: &'static TaskVTable,
}

impl Header {
    fn new(vtable: &'static TaskVTable, rt: &Arc<Runtime>) -> Self {
        Self {
            state: AtomicUsize::new(SLEEPING),
            refer: AtomicUsize::new(0),
            waker: AtomicWaker::new(),
            rt: Arc::clone(rt),
            queue_id: AtomicUsize::new(rt.local_queues.len()),
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

/// The [`TaskLayout`] struct defines the memory layout for a once-task.
///
/// It includes the combined [`Layout`] of the [`Header`] and the [`Data`] and
/// the offset at which the [`Data`] starts within the layout.
struct TaskLayout {
    layout: Layout,
    offset_data: usize,
}

impl TaskLayout {
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

/// The [`TaskVTable`] struct contains function pointers for operations on [`Task`].
///
/// It includes functions for retrieving the output of a completed task,
/// destroying the task, running the task, and scheduling the task.
struct TaskVTable {
    get_output: unsafe fn(*const ()) -> *const (),
    destroy: unsafe fn(*const ()),
    run: unsafe fn(*const (), usize),
    schedule: unsafe fn(*const ()),
}

/// The [`Data`] union holds the closure to be executed or the output of the closure.
union Data<F, T> {
    future: ManuallyDrop<F>,
    output: ManuallyDrop<T>,
}

/// The [`RawTask`] struct is a low-level representation of a task.
///
/// It contains pointers to the [`Header`] and [`Data`] of the task.
struct RawTask<F, T> {
    header: *const Header,
    data: *mut Data<F, T>,
}

// Impl const `RawTask`
impl<F, T> RawTask<F, T> {
    const LAYOUT: TaskLayout = TaskLayout::new::<F, T>();

    #[inline]
    const fn from_ptr(ptr: *const ()) -> Self {
        let ptr = ptr as *const u8;

        Self {
            header: ptr as *const Header,
            data: unsafe { ptr.add(Self::LAYOUT.offset_data) } as *mut Data<F, T>,
        }
    }
}

//Impl `RawWakerVTable` for `RawTask`
impl<F, T> RawTask<F, T>
where
    F: Future<Output = T> + Send + 'static,
{
    const VTABLE: RawWakerVTable = RawWakerVTable::new(
        Self::clone_waker,
        Self::wake,
        Self::wake_by_ref,
        Self::drop_waker,
    );

    #[inline]
    unsafe fn clone_waker(ptr: *const ()) -> RawWaker {
        let raw = Self::from_ptr(ptr);

        (*raw.header).increment_refer();

        RawWaker::new(ptr, &Self::VTABLE)
    }

    #[inline]
    unsafe fn wake(ptr: *const ()) {
        Self::wake_by_ref(ptr);
        Self::drop_waker(ptr);
    }

    #[inline]
    unsafe fn wake_by_ref(ptr: *const ()) {
        let raw = Self::from_ptr(ptr);
        let header = &*raw.header;

        match header
            .state
            .compare_exchange(SLEEPING, SCHEDULED, AcqRel, Acquire)
        {
            Ok(_) => {
                (header.vtable.schedule)(ptr);
            }
            Err(RUNNING) => {
                if header
                    .state
                    .compare_exchange(RUNNING, SCHEDULED, AcqRel, Acquire)
                    .is_err()
                {
                    return;
                }
            }
            Err(_) => {
                return;
            }
        }

        let queue = header.queue_id.load(Acquire);
        let handle = Task::from_raw_unchecked(NonNull::new(ptr as *mut ()).unwrap());

        if queue != header.rt.local_queues.len() {
            if let Err(err) = header.rt.local_queues[queue].push(handle) {
                let handle = err.into_inner();
                debug_assert!(header.rt.global_queue.push(handle).is_ok());
            }
        } else {
            debug_assert!(header.rt.global_queue.push(handle).is_ok());
        }
    }

    #[inline]
    unsafe fn drop_waker(ptr: *const ()) {
        let raw = Self::from_ptr(ptr);

        (*raw.header).decrement_refer(ptr);
    }
}

// Impl `TaskVTable` for `RawTask`
impl<F, T> RawTask<F, T>
where
    F: Future<Output = T> + Send + 'static,
{
    unsafe fn drop_future(ptr: *const ()) {
        let raw = Self::from_ptr(ptr);

        ManuallyDrop::drop(&mut (*raw.data).future);
    }

    unsafe fn get_output(ptr: *const ()) -> *const () {
        let raw = Self::from_ptr(ptr);

        (&*(*raw.data).output) as *const T as *const ()
    }

    unsafe fn destroy(ptr: *const ()) {
        let raw = Self::from_ptr(ptr);
        let header = &*raw.header;

        match header.state.load(Acquire) {
            COMPLETED => ManuallyDrop::drop(&mut (*raw.data).output),
            SLEEPING | SCHEDULED => Self::drop_future(ptr),
            _ => {}
        }

        (raw.header as *mut Header).drop_in_place();
        raw.data.drop_in_place();

        alloc::dealloc(ptr as *mut u8, Self::LAYOUT.layout);
    }

    unsafe fn run(ptr: *const (), queue_id: usize) {
        let raw = Self::from_ptr(ptr);
        let header = &*raw.header;

        if let Err(cur) = header
            .state
            .compare_exchange(SCHEDULED, RUNNING, AcqRel, Acquire)
        {
            unreachable!("invalid run task state: {}", cur);
        }

        header.queue_id.store(queue_id, Release);

        let future = Pin::new_unchecked(&mut *(*raw.data).future);

        let waker = ManuallyDrop::new(Waker::from_raw(RawWaker::new(ptr, &Self::VTABLE)));
        let cx = &mut Context::from_waker(&waker);

        match future.poll(cx) {
            Poll::Ready(output) => {
                header.state.store(COMPLETED, Release);
                Self::drop_future(ptr);

                header.rt.tasks.fetch_sub(1, AcqRel);

                (*raw.data).output = ManuallyDrop::new(output);

                header.decrement_refer(ptr);

                header.waker.wake();
            }
            Poll::Pending => {
                let _ = header
                    .state
                    .compare_exchange(RUNNING, SLEEPING, AcqRel, Acquire);
            }
        }
    }
}

impl<F, T> RawTask<F, T>
where
    F: Future<Output = T> + Send + 'static,
{
    /// Allocates a new once-task based on [`TaskLayout`] and the given closure.
    ///
    /// Return a pointer to the task.
    fn allocate(future: F, rt: &Arc<Runtime>) -> NonNull<()> {
        unsafe {
            let ptr = match NonNull::new(alloc::alloc(Self::LAYOUT.layout) as *mut ()) {
                Some(ptr) => ptr,
                None => alloc::handle_alloc_error(Self::LAYOUT.layout),
            };

            let raw = Self::from_ptr(ptr.as_ptr());

            (raw.header as *mut Header).write(Header::new(
                &TaskVTable {
                    get_output: Self::get_output,
                    destroy: Self::destroy,
                    run: Self::run,
                    schedule: Self::wake_by_ref,
                },
                rt,
            ));

            raw.data.write(Data {
                future: ManuallyDrop::new(future),
            });

            ptr
        }
    }
}

/// The [`Task`] struct represents a task.
///
/// It is used by [`crate::executor`].
pub(crate) struct Task {
    ptr: NonNull<()>,
}

unsafe impl Send for Task {}
unsafe impl Sync for Task {}

impl Task {
    fn from_raw(ptr: NonNull<()>) -> Self {
        unsafe { (*(ptr.as_ptr() as *const Header)).increment_refer() };

        Self { ptr }
    }

    unsafe fn from_raw_unchecked(ptr: NonNull<()>) -> Self {
        Self { ptr }
    }

    pub(crate) fn run(self, queue_id: usize) {
        let ptr = self.ptr.as_ptr();
        let header = ptr as *const Header;
        mem::forget(self);

        unsafe { ((*header).vtable.run)(ptr, queue_id) };
    }

    pub(crate) fn schedule(self) {
        let ptr = self.ptr.as_ptr();
        let header = unsafe { &*(ptr as *const Header) };
        mem::forget(self);

        header.rt.tasks.fetch_add(1, AcqRel);

        unsafe { (header.vtable.schedule)(ptr) };
    }
}

impl Drop for Task {
    fn drop(&mut self) {
        let ptr = self.ptr.as_ptr();

        unsafe { (*(ptr as *const Header)).decrement_refer(ptr) };
    }
}

/// The [`TaskHandle`] struct is a handle to a [`Task`].
///
/// It implements [`Future`] and can be used to poll the task and retrieve its output.
pub struct TaskHandle<T> {
    ptr: NonNull<()>,
    _marker: PhantomData<T>,
}

unsafe impl<T: Send> Send for TaskHandle<T> {}
unsafe impl<T: Sync> Sync for TaskHandle<T> {}

impl<T> Unpin for TaskHandle<T> {}

impl<T> TaskHandle<T> {
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

impl<T> Drop for TaskHandle<T> {
    fn drop(&mut self) {
        let ptr = self.ptr.as_ptr();

        unsafe { (*(ptr as *const Header)).decrement_refer(ptr) };
    }
}

impl<T> Future for TaskHandle<T> {
    type Output = T;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.poll_task(cx) {
            Poll::Ready(t) => Poll::Ready(t.expect("task is closed")),
            Poll::Pending => Poll::Pending,
        }
    }
}
