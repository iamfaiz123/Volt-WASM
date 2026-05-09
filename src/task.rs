use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

/// A spawned unit of work inside the executor.
///
/// Wraps a type-erased, pinned, boxed future. No `Send` bound is required
/// because Volt is a single-threaded runtime designed for WASM — `!Send`
/// futures (such as those holding JS objects) are explicitly supported.
pub struct Task {
    future: Pin<Box<dyn Future<Output = ()>>>,
}

impl Task {
    /// Create a new task from any `Future<Output = ()> + 'static`.
    pub fn new<F>(future: F) -> Self
    where
        F: Future<Output = ()> + 'static,
    {
        Task {
            future: Box::pin(future),
        }
    }

    /// Poll the inner future.
    pub fn poll(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        self.future.as_mut().poll(cx)
    }
}

// ------------------------------------------------------------------
// Tests
// ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::task::Wake;
    use std::sync::Arc;

    fn dummy_waker() -> std::task::Waker {
        struct DummyWake;
        impl Wake for DummyWake {
            fn wake(self: Arc<Self>) {}
        }
        std::task::Waker::from(Arc::new(DummyWake))
    }

    #[test]
    fn test_task_creation() {
        let _task = Task::new(async { println!("Hello"); });
    }

    #[test]
    fn test_task_poll_completes() {
        let mut task = Task::new(async {});
        let waker = dummy_waker();
        let mut cx = Context::from_waker(&waker);
        let result = task.poll(&mut cx);
        assert!(matches!(result, Poll::Ready(())));
    }
}
