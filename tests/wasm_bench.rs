//! WASM timing tests — run with: `wasm-pack test --node`
//!
//! These tests measure raw scheduling throughput in a real WASM environment
//! (Node.js / V8), which is the actual target for the Volt runtime.
//!
//! Criterion does NOT run on WASM, so we use `js_sys::Date::now()` for
//! millisecond-precision wall-clock timing and print results via `println!`
//! (captured by wasm-bindgen-test's output harness).
//!
//! Results show up in the test output as:
//!   [volt/1k]     X.XX ms  (Y ns/task)
//!   [volt/10k]    X.XX ms  (Y ns/task)
//!   [volt/100k]   X.XX ms  (Y ns/task)
//!   [volt/1M]     X.XX ms  (Y ns/task)

// Only compile this file when targeting WASM.
#![cfg(target_arch = "wasm32")]

use wasm_bindgen_test::*;

/// Measure current time in milliseconds via the JS `Date` API.
/// This works in both browser and Node.js environments.
fn now_ms() -> f64 {
    js_sys::Date::now()
}

/// Spawn `n` immediately-ready tasks on Volt and run to completion.
/// Returns wall-clock time in milliseconds.
fn time_volt(n: usize) -> f64 {
    let t0 = now_ms();
    let mut rt = volt::Runtime::new();
    for _ in 0..n {
        rt.spawn(async {});
    }
    rt.run();
    now_ms() - t0
}

fn print_result(label: &str, n: usize, ms: f64) {
    let ns_per_task = (ms * 1_000_000.0) / n as f64;
    let msg = format!("[{label}]  {ms:.3} ms  ({ns_per_task:.0} ns/task)");
    println!("{}", msg);
}

// ── Volt timing tests ─────────────────────────────────────────────────────────

#[wasm_bindgen_test]
fn wasm_bench_volt_1k() {
    let ms = time_volt(1_000);
    print_result("volt/1k", 1_000, ms);
    // Not a strict assert — we just want it to complete without crashing.
    assert!(ms < 120_000.0);
}

#[wasm_bindgen_test]
fn wasm_bench_volt_10k() {
    let ms = time_volt(10_000);
    print_result("volt/10k", 10_000, ms);
    assert!(ms < 120_000.0);
}

#[wasm_bindgen_test]
fn wasm_bench_volt_100k() {
    let ms = time_volt(100_000);
    print_result("volt/100k", 100_000, ms);
    assert!(ms < 120_000.0);
}

#[wasm_bindgen_test]
fn wasm_bench_volt_1m() {
    let ms = time_volt(1_000_000);
    print_result("volt/1M", 1_000_000, ms);
    assert!(ms < 120_000.0);
}

// ── Correctness smoke test ────────────────────────────────────────────────────

#[wasm_bindgen_test]
fn wasm_tasks_actually_run() {
    use std::cell::Cell;
    use std::rc::Rc;

    let counter = Rc::new(Cell::new(0usize));
    let mut rt = volt::Runtime::new();

    for _ in 0..100 {
        let c = counter.clone();
        rt.spawn(async move { c.set(c.get() + 1); });
    }
    rt.run();

    assert_eq!(counter.get(), 100, "not all tasks ran!");
}
