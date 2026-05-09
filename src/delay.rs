use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};
use std::time::{Duration, Instant};

/// A timer-based future that resolves after a specified duration.
///
/// ## Implementation note
///
/// On **native** (non-WASM) targets this uses a busy-poll strategy: each time
/// the future is polled and the deadline has not passed yet, it immediately
/// schedules itself for re-polling via `wake_by_ref()`. This burns CPU but
/// requires zero threads and is the correct structure for demonstrating the
/// waker pattern in a single-threaded context.
///
/// On **WASM** you would replace the `wake_by_ref()` call with a call to the
/// JavaScript `setTimeout` API (e.g. via `gloo-timers` or `wasm-bindgen`),
/// which posts a microtask that calls `wake()` after the delay — zero threads,
/// zero busy-polling.
pub struct Delay {
    deadline: Instant,
    /// The latest waker handed to us by the executor. We keep it so a
    /// hypothetical timer callback could call `wake()` on it.
    waker: Option<Waker>,
}

impl Unpin for Delay {}

impl Delay {
    /// Create a new `Delay` that completes after `duration` from now.
    pub fn new(duration: Duration) -> Self {
        Delay {
            deadline: Instant::now() + duration,
            waker: None,
        }
    }
}

impl Future for Delay {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();

        if Instant::now() >= this.deadline {
            log::trace!("[Delay] Ready");
            Poll::Ready(())
        } else {
            // Store the latest waker so an external timer callback could use it.
            this.waker = Some(cx.waker().clone());

            // Busy-poll: immediately reschedule ourselves so the executor re-polls
            // us on the next tick.  In a real WASM runtime this line is replaced
            // by registering a JS setTimeout callback.
            cx.waker().wake_by_ref();

            log::trace!("[Delay] Pending (busy-poll)");
            Poll::Pending
        }
    }
}

// ------------------------------------------------------------------
// Tests
// ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::task::Wake;
    use std::thread;

    fn dummy_waker() -> std::task::Waker {
        struct DummyWake;
        impl Wake for DummyWake {
            fn wake(self: Arc<Self>) {}
        }
        std::task::Waker::from(Arc::new(DummyWake))
    }

    #[test]
    fn test_delay_future_ready_immediately() {
        let delay = Delay::new(Duration::from_nanos(1));
        thread::sleep(Duration::from_millis(1));

        let waker = dummy_waker();
        let mut cx = Context::from_waker(&waker);
        let mut delay = Box::pin(delay);
        let result = delay.as_mut().poll(&mut cx);
        assert!(matches!(result, Poll::Ready(())));
    }

    #[test]
    fn test_delay_future_pending() {
        let delay = Delay::new(Duration::from_secs(60));
        let waker = dummy_waker();
        let mut cx = Context::from_waker(&waker);
        let mut delay = Box::pin(delay);
        let result = delay.as_mut().poll(&mut cx);
        assert!(matches!(result, Poll::Pending));
    }
}
