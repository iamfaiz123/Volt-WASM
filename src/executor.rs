use std::cell::RefCell;
use std::collections::VecDeque;
use std::future::Future;
use std::rc::Rc;
use std::task::{Context, Poll};

use crate::arena::{GenerationalArena, TaskId};
use crate::task::Task;
use crate::waker::make_waker;

/// A single-threaded async task executor designed for WASM.
///
/// All interior shared state uses `Rc<RefCell<...>>` instead of
/// `Arc<Mutex<...>>` — no locking overhead, no Send requirements, no threads.
///
/// ## Scheduling
///
/// ```text
///   spawn(future)
///       └─► insert into GenerationalArena → push TaskId to ready_queue
///
///   tick()
///       ├─ drain shared_ready (waker inbox) → ready_queue
///       └─ for each id in ready_queue:
///              poll(task)
///               ├─ Ready  → remove from arena
///               └─ Pending → waker will re-enqueue id later
/// ```
pub struct Executor {
    tasks: GenerationalArena<Task>,
    /// Local queue — only touched by the executor on the same thread.
    ready_queue: VecDeque<TaskId>,
    /// Shared "inbox": wakers push here; executor drains it each tick.
    /// `Rc<RefCell<...>>` is safe because everything runs on one thread.
    shared_ready: Rc<RefCell<Vec<TaskId>>>,
}

impl Executor {
    /// Create a new, empty executor.
    pub fn new() -> Self {
        Executor {
            tasks: GenerationalArena::new(),
            ready_queue: VecDeque::new(),
            shared_ready: Rc::new(RefCell::new(Vec::new())),
        }
    }

    /// Spawn a future as a new task and return its [`TaskId`].
    ///
    /// No `Send` bound — `!Send` futures (e.g. holding JS objects) are valid.
    pub fn spawn<F>(&mut self, future: F) -> TaskId
    where
        F: Future<Output = ()> + 'static,
    {
        let task = Task::new(future);
        let id = self.tasks.insert(task);
        self.ready_queue.push_back(id);
        log::trace!("[Executor] Spawned task {:?}", id);
        id
    }

    /// Run a single scheduling cycle:
    ///
    /// 1. Drain the waker inbox (`shared_ready`) into the local `ready_queue`.
    /// 2. Poll every task in `ready_queue`.
    /// 3. Remove completed tasks from the arena.
    pub fn tick(&mut self) {
        log::trace!("=== TICK START === (active: {})", self.tasks.len());

        // 1. Drain waker inbox — one borrow, then released before any polling.
        {
            let mut inbox = self.shared_ready.borrow_mut();
            log::trace!("Woken tasks: {}", inbox.len());
            for id in inbox.drain(..) {
                if self.tasks.contains(id) {
                    self.ready_queue.push_back(id);
                }
            }
        } // ← RefCell borrow released here

        // 2. Poll all ready tasks.
        let task_ids: Vec<TaskId> = self.ready_queue.drain(..).collect();
        for task_id in task_ids {
            let waker = make_waker(task_id, self.shared_ready.clone());
            let mut cx = Context::from_waker(&waker);

            let poll_result = if let Some(task) = self.tasks.get_mut(task_id) {
                log::trace!("Polling task {:?}", task_id);
                task.poll(&mut cx)
            } else {
                continue; // stale id (ABA protection fired)
            };

            match poll_result {
                Poll::Ready(()) => {
                    log::trace!("Task {:?} completed", task_id);
                    self.tasks.remove(task_id);
                }
                Poll::Pending => {
                    log::trace!("Task {:?} pending", task_id);
                }
            }
        }

        log::trace!("=== TICK END === (active: {})", self.tasks.len());
    }

    /// Drive the executor until all tasks have completed.
    pub fn run(&mut self) {
        while !self.tasks.is_empty() {
            self.tick();
        }
    }

    /// Returns `true` if there are no active tasks.
    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }

    /// Returns the number of active tasks.
    pub fn task_count(&self) -> usize {
        self.tasks.len()
    }
}

impl Default for Executor {
    fn default() -> Self {
        Self::new()
    }
}

// ------------------------------------------------------------------
// Tests
// ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spawn_creates_task() {
        let mut executor = Executor::new();
        let id1 = executor.spawn(async {});
        let id2 = executor.spawn(async {});
        assert_ne!(id1, id2);
        assert!(!executor.is_empty());
    }

    #[test]
    fn test_tick_processes_ready_task() {
        let mut executor = Executor::new();
        executor.spawn(async {});
        assert!(!executor.is_empty());
        executor.tick();
        assert!(executor.is_empty());
    }

    #[test]
    fn test_multiple_tasks_execute() {
        let mut executor = Executor::new();
        for _ in 0..3 {
            executor.spawn(async {});
        }
        assert_eq!(executor.task_count(), 3);
        executor.run();
        assert_eq!(executor.task_count(), 0);
    }
}
