use volt::{Delay, Runtime};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[test]
fn test_single_task_execution() {
    let executed = Arc::new(Mutex::new(false));
    let executed_clone = executed.clone();

    let mut runtime = Runtime::new();
    runtime.spawn(async move {
        *executed_clone.lock().unwrap() = true;
    });
    runtime.run();

    assert!(*executed.lock().unwrap());
}

#[test]
fn test_multiple_concurrent_tasks() {
    let counter = Arc::new(Mutex::new(0));

    let mut runtime = Runtime::new();

    for _ in 0..5 {
        let c = counter.clone();
        runtime.spawn(async move {
            *c.lock().unwrap() += 1;
        });
    }

    runtime.run();

    assert_eq!(*counter.lock().unwrap(), 5);
}

#[test]
fn test_delay_waits_correctly() {
    let mut runtime = Runtime::new();

    let ok = Arc::new(Mutex::new(false));
    let ok_clone = ok.clone();

    runtime.spawn(async move {
        let start = Instant::now();
        Delay::new(Duration::from_millis(100)).await;
        let elapsed = start.elapsed();

        // Should have waited at least 100ms.
        assert!(
            elapsed >= Duration::from_millis(100),
            "elapsed was only {:?}",
            elapsed
        );
        // And shouldn't overshoot too much (allow 150ms slack for CI).
        assert!(
            elapsed < Duration::from_millis(250),
            "elapsed was {:?}",
            elapsed
        );

        *ok_clone.lock().unwrap() = true;
    });

    runtime.run();
    assert!(*ok.lock().unwrap(), "delay task never completed");
}

#[test]
fn test_join_waits_for_both() {
    let mut runtime = Runtime::new();

    let ok = Arc::new(Mutex::new(false));
    let ok_clone = ok.clone();

    runtime.spawn(async move {
        let delay1 = Delay::new(Duration::from_millis(50));
        let delay2 = Delay::new(Duration::from_millis(100));

        let start = Instant::now();
        volt::future::join(delay1, delay2).await;
        let elapsed = start.elapsed();

        // Should wait for the longer delay.
        assert!(
            elapsed >= Duration::from_millis(100),
            "elapsed was only {:?}",
            elapsed
        );

        *ok_clone.lock().unwrap() = true;
    });

    runtime.run();
    assert!(*ok.lock().unwrap(), "join task never completed");
}

#[test]
fn test_select_returns_first() {
    let order = Arc::new(Mutex::new(Vec::new()));
    let mut runtime = Runtime::new();

    let order_clone = order.clone();
    runtime.spawn(async move {
        let delay1 = Delay::new(Duration::from_millis(200));
        let delay2 = Delay::new(Duration::from_millis(50));

        match volt::future::select(delay1, delay2).await {
            volt::future::Either::Left(_) => {
                order_clone.lock().unwrap().push("first");
            }
            volt::future::Either::Right(_) => {
                order_clone.lock().unwrap().push("second");
            }
        }
    });

    runtime.run();

    // The 50ms delay (second arg → Right) should complete first.
    assert_eq!(order.lock().unwrap()[0], "second");
}

#[test]
fn test_nested_futures() {
    let result = Arc::new(Mutex::new(0));
    let result_clone = result.clone();

    let mut runtime = Runtime::new();

    runtime.spawn(async move {
        async {
            async {
                *result_clone.lock().unwrap() = 42;
            }
            .await
        }
        .await
    });

    runtime.run();

    assert_eq!(*result.lock().unwrap(), 42);
}
