// use std::alloc::Layout;

// use std::future::Future;
// use std::mem::ManuallyDrop;
// use std::pin::Pin;
// use std::ptr;
// use std::ptr::NonNull;
// use std::sync::atomic::AtomicUsize;
// use std::sync::atomic::Ordering::*;
// use std::task::{Context, Poll};

// use std::task::{RawWaker, RawWakerVTable, Waker};

// use std::sync::Arc;

// use crate::waker::AtomicWaker;

// use super::header::Header;
// use super::task::Task;

// pub(crate) struct TaskVTable {
//     pub(crate) schedule: unsafe fn(*const ()),
//     pub(crate) drop_future: unsafe fn(*const ()),
//     pub(crate) get_output: unsafe fn(*const ()) -> *const (),
//     pub(crate) increment_ref_count: unsafe fn(*const ()),
//     pub(crate) decrement_ref_count: unsafe fn(*const ()),
//     pub(crate) destroy: unsafe fn(*const ()),
//     pub(crate) run: unsafe fn(*const ()),
//     pub(crate) clone_waker: unsafe fn(*const ()) -> RawWaker,
//     pub(crate) layout: &'static TaskLayout,
// }

// pub(crate) struct TaskLayout {
//     pub(crate) layout: Layout,
//     pub(crate) offset_s: usize,
//     pub(crate) offset_f: usize,
//     pub(crate) offset_o: usize,
// }

// impl TaskLayout {
//     const fn new<S, F, O>() -> Self {
//         let layout_s = Layout::new::<S>();
//         let layout_f = Layout::new::<F>();
//         let layout_o = Layout::new::<O>();

//         let size_fo = layout_f.size().max(layout_o.size());
//         let align_fo = layout_f.align().max(layout_o.align());
//         let Ok(layout_fo) = Layout::from_size_align(size_fo, align_fo) else {
//             panic!("layout_fo is not valid");
//         };

//         let layout = Layout::new::<Header>();
//         let (layout, offset_s) = Self::extend(layout, layout_s);
//         let (layout, offset_f) = Self::extend(layout, layout_fo);
//         let offset_o = offset_f;

//         Self {
//             layout,
//             offset_s,
//             offset_f,
//             offset_o,
//         }
//     }

//     const fn extend(pre: Layout, nex: Layout) -> (Layout, usize) {
//         let align = pre.align().max(nex.align());
//         let pad = pre.padding_needed_for(nex.align());

//         let Some(offset) = pre.size().checked_add(pad) else {
//             panic!("layout is not valid");
//         };
//         let Some(size) = offset.checked_add(nex.size()) else {
//             panic!("layout is not valid");
//         };

//         (
//             unsafe { Layout::from_size_align_unchecked(size, align) },
//             offset,
//         )
//     }
// }

// pub(crate) struct RawTask<S, F, O> {
//     pub(crate) header: *const Header,
//     pub(crate) schedule: *const S,
//     pub(crate) future: *mut F,
//     pub(crate) output: *const O,
// }

// impl<S, F, O> RawTask<S, F, O> {
//     const LAYOUT: TaskLayout = TaskLayout::new::<S, F, O>();
// }

// impl<S, F, O> RawTask<S, F, O>
// where
//     F: Future<Output = O>,
//     S: Fn(Task),
// {
//     const VTABLE: RawWakerVTable = RawWakerVTable::new(
//         Self::clone_waker,
//         Self::wake,
//         Self::wake_by_ref,
//         Self::drop_waker,
//     );

//     fn from_ptr(ptr: *const ()) -> Self {
//         let ptr = ptr as *const u8;
//         unsafe {
//             Self {
//                 header: ptr as *const Header,
//                 schedule: ptr.add(Self::LAYOUT.offset_s) as *const S,
//                 future: ptr.add(Self::LAYOUT.offset_f) as *mut F,
//                 output: ptr.add(Self::LAYOUT.offset_o) as *const O,
//             }
//         }
//     }

//     unsafe fn clone_waker(ptr: *const ()) -> RawWaker {
//         let raw = Self::from_ptr(ptr);

//         (*raw.header).increment_ref_count();

//         RawWaker::new(ptr, &Self::VTABLE)
//     }

//     unsafe fn wake(ptr: *const ()) {
//         let raw = Self::from_ptr(ptr);

//         (*raw.header).wake();
//     }
// }
