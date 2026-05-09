use std::future::Future;

use crate::arena::TaskId;
use crate::executor::Executor;

/// High-level single-threaded runtime — a thin wrapper around [`Executor`].
///
/// ```rust
/// use volt_wasm::Runtime;
///
/// let mut rt = Runtime::new();
/// rt.spawn(async { println!("hello from volt!"); });
/// rt.run();
/// ```
pub struct Runtime {
    executor: Executor,
}

impl Runtime {
    /// Create a new single-threaded runtime.
    pub fn new() -> Self {
        Runtime {
            executor: Executor::new(),
        }
    }

    /// Spawn a future onto the runtime.
    ///
    /// No `Send` bound required — this runtime is single-threaded.
    pub fn spawn<F>(&mut self, future: F) -> TaskId
    where
        F: Future<Output = ()> + 'static,
    {
        self.executor.spawn(future)
    }

    /// Run until all spawned tasks complete.
    pub fn run(&mut self) {
        self.executor.run();
    }
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
}
