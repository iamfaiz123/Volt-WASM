use std::cell::RefCell;
use std::rc::Rc;
use std::task::{RawWaker, RawWakerVTable, Waker};

use crate::arena::TaskId;

/// The data heap-allocated for every `Waker`.
///
/// We use `Rc<RefCell<...>>` instead of `Arc<Mutex<...>>` because the runtime
/// is strictly single-threaded (designed for WASM). `Rc` is not `Send`, which
/// is intentional — it prevents accidental use across threads.
///
/// Because `Rc` is `!Send`, we cannot use the `std::task::Wake` trait (which
/// requires `Arc<Self>`). Instead, we drop down to the lower-level
/// `RawWaker` + `RawWakerVTable` API.
struct LocalWakerData {
    task_id: TaskId,
    ready_tasks: Rc<RefCell<Vec<TaskId>>>,
}

// ------------------------------------------------------------------
// RawWaker vtable functions
// Each function receives a `*const ()` that is really a
// `*const LocalWakerData` owned by an `Rc`.
// ------------------------------------------------------------------

unsafe fn vtable_clone(data: *const ()) -> RawWaker {
    let rc = unsafe { Rc::from_raw(data as *const LocalWakerData) };
    let cloned = Rc::clone(&rc);
    std::mem::forget(rc);
    RawWaker::new(Rc::into_raw(cloned) as *const (), &VTABLE)
}

unsafe fn vtable_wake(data: *const ()) {
    let rc = unsafe { Rc::from_raw(data as *const LocalWakerData) };
    rc.ready_tasks.borrow_mut().push(rc.task_id);
}

unsafe fn vtable_wake_by_ref(data: *const ()) {
    let rc = unsafe { Rc::from_raw(data as *const LocalWakerData) };
    rc.ready_tasks.borrow_mut().push(rc.task_id);
    std::mem::forget(rc);
}

unsafe fn vtable_drop(data: *const ()) {
    drop(unsafe { Rc::from_raw(data as *const LocalWakerData) });
}

static VTABLE: RawWakerVTable =
    RawWakerVTable::new(vtable_clone, vtable_wake, vtable_wake_by_ref, vtable_drop);

/// Create a `Waker` for `task_id` that pushes into `ready_tasks` on wake.
///
/// This is the WASM-safe, `!Send` equivalent of the old `MinimalWaker`.
pub fn make_waker(task_id: TaskId, ready_tasks: Rc<RefCell<Vec<TaskId>>>) -> Waker {
    let data = Rc::new(LocalWakerData {
        task_id,
        ready_tasks,
    });
    let ptr = Rc::into_raw(data) as *const ();
    // SAFETY: vtable functions correctly manage the Rc reference count,
    // and this waker is only ever used on the single thread that created it.
    unsafe { Waker::from_raw(RawWaker::new(ptr, &VTABLE)) }
}

// ------------------------------------------------------------------
// Tests
// ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_waker_adds_to_queue() {
        let ready_tasks = Rc::new(RefCell::new(Vec::new()));
        let waker = make_waker(TaskId { index: 5, generation: 0 }, ready_tasks.clone());

        assert_eq!(ready_tasks.borrow().len(), 0);
        waker.wake_by_ref();
        assert_eq!(ready_tasks.borrow().len(), 1);
        assert_eq!(ready_tasks.borrow()[0], TaskId { index: 5, generation: 0 });
    }

    #[test]
    fn test_waker_wake_consumes() {
        let ready_tasks = Rc::new(RefCell::new(Vec::new()));
        let waker = make_waker(TaskId { index: 10, generation: 0 }, ready_tasks.clone());

        waker.wake(); // consumes waker
        assert_eq!(ready_tasks.borrow()[0], TaskId { index: 10, generation: 0 });
    }

    #[test]
    fn test_waker_clone() {
        let ready_tasks = Rc::new(RefCell::new(Vec::new()));
        let waker = make_waker(TaskId { index: 7, generation: 2 }, ready_tasks.clone());
        let waker2 = waker.clone();

        waker.wake_by_ref();
        waker2.wake_by_ref();

        assert_eq!(ready_tasks.borrow().len(), 2);
        assert_eq!(ready_tasks.borrow()[0], TaskId { index: 7, generation: 2 });
        assert_eq!(ready_tasks.borrow()[1], TaskId { index: 7, generation: 2 });
    }
}
