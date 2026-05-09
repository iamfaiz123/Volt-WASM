//! Native benchmark: spawn N immediately-ready tasks and run to completion.
//!
//! ## Fairness notes
//!
//! Every runtime uses the same logical pattern:
//!   "spawn N tasks that each do `black_box(())`; drive executor to empty"
//!
//! | Runtime                  | Spawn API            | Wait-for-all pattern      |
//! |--------------------------|----------------------|---------------------------|
//! | **Volt**                 | `rt.spawn()`         | `rt.run()` (drains arena) |
//! | **async-executor local** | `ex.spawn().detach()`| `ex.run(barrier)` + counter |
//! | **futures LocalPool**    | `spawner.spawn_local`| `pool.run()` (drains pool)|
//! | **Tokio current_thread** | `spawn_local()`      | `join_all(handles)`       |
//! | **pollster**             | no spawn API         | `join_all(raw futures)`   |
//!
//! Tokio collects `JoinHandle`s because there is no `run_until_empty` API on
//! `LocalSet`. The `Vec::with_capacity` pre-alloc makes this overhead O(1) per
//! iteration, not O(N). This is documented, not hidden.
//!
//! Pollster has **no multi-task spawn API** at all. It is included to show the
//! cost of sequentially polling N futures via `join_all` with zero scheduling
//! overhead — a lower bound on "what does just polling N futures cost".

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

const COUNTS: &[u64] = &[1_000, 10_000, 100_000, 1_000_000];

// ── 1. Volt ───────────────────────────────────────────────────────────────────
fn bench_volt(c: &mut Criterion) {
    let mut g = c.benchmark_group("volt");
    g.sample_size(10);

    for &n in COUNTS {
        g.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
            b.iter(|| {
                let mut rt = volt_wasm::Runtime::new();
                for _ in 0..n {
                    rt.spawn(async { black_box(()); });
                }
                rt.run();
            });
        });
    }
    g.finish();
}

// ── 2. async-executor LocalExecutor (smol) ───────────────────────────────────
fn bench_async_executor(c: &mut Criterion) {
    use async_executor::LocalExecutor;
    use futures::executor::block_on;
    use futures::future::join_all;

    let mut g = c.benchmark_group("async_executor_local");
    g.sample_size(10);

    for &n in COUNTS {
        g.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
            b.iter(|| {
                let ex = LocalExecutor::new();
                // Spawn N tasks inside ex.run; collect Task handles then join_all.
                // This is the canonical smol pattern.
                block_on(ex.run(async {
                    let tasks: Vec<_> = (0..n)
                        .map(|_| ex.spawn(async { black_box(()); }))
                        .collect();
                    join_all(tasks).await;
                }));
            });
        });
    }
    g.finish();
}

// ── 3. futures::executor::LocalPool ──────────────────────────────────────────
fn bench_futures_local_pool(c: &mut Criterion) {
    use futures::executor::LocalPool;
    use futures::task::LocalSpawnExt;

    let mut g = c.benchmark_group("futures_local_pool");
    g.sample_size(10);

    for &n in COUNTS {
        g.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
            b.iter(|| {
                let mut pool = LocalPool::new();
                let spawner = pool.spawner();
                for _ in 0..n {
                    spawner.spawn_local(async { black_box(()); }).unwrap();
                }
                pool.run();
            });
        });
    }
    g.finish();
}

// ── 4. Tokio current_thread + LocalSet ───────────────────────────────────────
fn bench_tokio_current_thread(c: &mut Criterion) {
    use futures::future::join_all;

    let mut g = c.benchmark_group("tokio_current_thread");
    g.sample_size(10);

    // Build runtime once — it carries an I/O driver and timer wheel even when
    // unused; that fixed overhead is intentionally included as it is always
    // present in a real Tokio deployment.
    let rt = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();

    for &n in COUNTS {
        g.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
            b.iter(|| {
                let local = tokio::task::LocalSet::new();
                local.block_on(&rt, async {
                    // Pre-allocate so Vec growth is O(1) amortized, not O(N).
                    let mut handles = Vec::with_capacity(n as usize);
                    for _ in 0..n {
                        handles.push(tokio::task::spawn_local(async { black_box(()); }));
                    }
                    join_all(handles).await;
                });
            });
        });
    }
    g.finish();
}

// ── 5. pollster (block_on only — no spawn) ────────────────────────────────────
fn bench_pollster(c: &mut Criterion) {
    use futures::future::join_all;

    let mut g = c.benchmark_group("pollster_join_all");
    g.sample_size(10);

    // NOTE: pollster has no spawn/task concept. This benchmark measures the
    // minimum cost of polling N futures cooperatively with join_all.
    // It shows the overhead of scheduling (none) vs pure future polling.
    for &n in COUNTS {
        g.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
            b.iter(|| {
                pollster::block_on(async {
                    let futs: Vec<_> = (0..n).map(|_| async { black_box(()); }).collect();
                    join_all(futs).await;
                });
            });
        });
    }
    g.finish();
}

// ─────────────────────────────────────────────────────────────────────────────
criterion_group!(
    benches,
    bench_volt,
    bench_async_executor,
    bench_futures_local_pool,
    bench_tokio_current_thread,
    bench_pollster,
);
criterion_main!(benches);
