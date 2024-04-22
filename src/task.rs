pub mod once_task;
pub use once_task::OnceTaskHandle;
pub(crate) use once_task::{task_and_handle as once_task_and_handle, OnceTask};

#[allow(clippy::module_inception)]
pub mod task;
pub use task::TaskHandle;
pub(crate) use task::{task_and_handle, Task};
