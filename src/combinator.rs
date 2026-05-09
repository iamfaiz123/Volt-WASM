use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

// ---------------------------------------------------------------------------
// Either
// ---------------------------------------------------------------------------

/// Result of a [`select`] — indicates which of the two futures completed first.
pub enum Either<A, B> {
    Left(A),
    Right(B),
}

// ---------------------------------------------------------------------------
// Join
// ---------------------------------------------------------------------------

/// A future that polls two inner futures and completes when **both** are done.
///
/// Created via the [`join`] free function.
pub struct Join<A, B>
where
    A: Future + Unpin,
    B: Future + Unpin,
{
    a: Option<A>,
    b: Option<B>,
    a_result: Option<A::Output>,
    b_result: Option<B::Output>,
}

// All fields are Unpin, so Join is safe to Unpin.
impl<A, B> Unpin for Join<A, B>
where
    A: Future + Unpin,
    B: Future + Unpin,
{
}

impl<A, B> Future for Join<A, B>
where
    A: Future + Unpin,
    B: Future + Unpin,
{
    type Output = (A::Output, B::Output);

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();

        // Poll A if it hasn't completed yet.
        if let Some(a) = this.a.as_mut() {
            if let Poll::Ready(val) = Pin::new(a).poll(cx) {
                this.a_result = Some(val);
                this.a = None;
            }
        }

        // Poll B if it hasn't completed yet.
        if let Some(b) = this.b.as_mut() {
            if let Poll::Ready(val) = Pin::new(b).poll(cx) {
                this.b_result = Some(val);
                this.b = None;
            }
        }

        // Return Ready only when both have produced a value.
        if this.a_result.is_some() && this.b_result.is_some() {
            let a = this.a_result.take().unwrap();
            let b = this.b_result.take().unwrap();
            Poll::Ready((a, b))
        } else {
            Poll::Pending
        }
    }
}

/// Wait for **both** futures to complete, returning a tuple of their results.
///
/// ```ignore
/// let (a, b) = volt::future::join(future_a, future_b).await;
/// ```
pub fn join<A, B>(a: A, b: B) -> Join<A, B>
where
    A: Future + Unpin,
    B: Future + Unpin,
{
    Join {
        a: Some(a),
        b: Some(b),
        a_result: None,
        b_result: None,
    }
}

// ---------------------------------------------------------------------------
// Select
// ---------------------------------------------------------------------------

/// A future that polls two inner futures and completes as soon as **either**
/// one is done, returning an [`Either`].
///
/// Created via the [`select`] free function.
pub struct Select<A, B>
where
    A: Future + Unpin,
    B: Future + Unpin,
{
    a: A,
    b: B,
}

impl<A, B> Unpin for Select<A, B>
where
    A: Future + Unpin,
    B: Future + Unpin,
{
}

impl<A, B> Future for Select<A, B>
where
    A: Future + Unpin,
    B: Future + Unpin,
{
    type Output = Either<A::Output, B::Output>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();

        // Poll A first.
        if let Poll::Ready(val) = Pin::new(&mut this.a).poll(cx) {
            return Poll::Ready(Either::Left(val));
        }

        // Then B.
        if let Poll::Ready(val) = Pin::new(&mut this.b).poll(cx) {
            return Poll::Ready(Either::Right(val));
        }

        Poll::Pending
    }
}

/// Return the result of whichever future completes **first**.
///
/// ```ignore
/// match volt::future::select(fast, slow).await {
///     Either::Left(val)  => { /* fast finished first */ }
///     Either::Right(val) => { /* slow finished first */ }
/// }
/// ```
pub fn select<A, B>(a: A, b: B) -> Select<A, B>
where
    A: Future + Unpin,
    B: Future + Unpin,
{
    Select { a, b }
}

// ---------------------------------------------------------------------------
// Map
// ---------------------------------------------------------------------------

/// A future that transforms the output of another future.
///
/// Created via the [`map`] free function.
pub struct Map<F, Func>
where
    F: Future + Unpin,
{
    future: F,
    f: Option<Func>,
}

impl<F, Func> Unpin for Map<F, Func> where F: Future + Unpin {}

impl<F, Func, T> Future for Map<F, Func>
where
    F: Future + Unpin,
    Func: FnOnce(F::Output) -> T + Unpin,
{
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        match Pin::new(&mut this.future).poll(cx) {
            Poll::Ready(val) => {
                let f = this.f.take().expect("Map polled after completion");
                Poll::Ready(f(val))
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Transform the output of `future` using the closure `f`.
///
/// ```ignore
/// let doubled = volt::future::map(async { 21 }, |x| x * 2).await;
/// assert_eq!(doubled, 42);
/// ```
pub fn map<F, Func, T>(future: F, f: Func) -> Map<F, Func>
where
    F: Future + Unpin,
    Func: FnOnce(F::Output) -> T + Unpin,
{
    Map {
        future,
        f: Some(f),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::task::Wake;

    fn dummy_waker() -> std::task::Waker {
        struct DummyWake;
        impl Wake for DummyWake {
            fn wake(self: Arc<Self>) {}
        }
        std::task::Waker::from(Arc::new(DummyWake))
    }

    #[test]
    fn test_join_both_ready() {
        // Box::pin async blocks to satisfy the Unpin bound (edition 2024).
        let a = Box::pin(async { 1u32 });
        let b = Box::pin(async { 2u32 });

        let mut joined = Box::pin(join(a, b));

        let waker = dummy_waker();
        let mut cx = Context::from_waker(&waker);

        // Both futures are immediately ready → Join should be Ready.
        if let Poll::Ready((va, vb)) = joined.as_mut().poll(&mut cx) {
            assert_eq!(va, 1);
            assert_eq!(vb, 2);
        } else {
            panic!("Expected Ready");
        }
    }

    #[test]
    fn test_select_returns_first_ready() {
        let a = Box::pin(async { "a" });
        let b = Box::pin(async { "b" });

        let mut sel = Box::pin(select(a, b));

        let waker = dummy_waker();
        let mut cx = Context::from_waker(&waker);

        // Both ready, but A is polled first → should get Left.
        if let Poll::Ready(Either::Left(val)) = sel.as_mut().poll(&mut cx) {
            assert_eq!(val, "a");
        } else {
            panic!("Expected Either::Left");
        }
    }

    #[test]
    fn test_map_transforms_output() {
        let future = Box::pin(async { 21 });
        let mapped = map(future, |x| x * 2);
        let mut mapped = Box::pin(mapped);

        let waker = dummy_waker();
        let mut cx = Context::from_waker(&waker);

        if let Poll::Ready(val) = mapped.as_mut().poll(&mut cx) {
            assert_eq!(val, 42);
        } else {
            panic!("Expected Ready");
        }
    }
}
