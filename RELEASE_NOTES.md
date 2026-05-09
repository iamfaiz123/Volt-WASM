# Volt v0.1.0-alpha: The Single-Threaded WASM Runtime ⚡

We are thrilled to announce the initial alpha release of **Volt v0.1.0-alpha**! 

Volt is a specialized, high-performance asynchronous runtime built specifically for single-threaded environments like WebAssembly (V8, SpiderMonkey) and embedded systems. 

## 🚀 Key Features

*   **Zero Atomics**: Completely stripped of `Arc`, `Mutex`, and multi-threaded synchronization primitives. Volt relies entirely on `Rc` and `RefCell` for memory management.
*   **WebAssembly Native**: Perfectly suited for `wasm32-unknown-unknown`. Because Volt is single-threaded, your futures do not require `Send` bounds, making it trivial to store `wasm_bindgen` JavaScript objects or DOM elements across `.await` points.
*   **Generational Arena Scheduling**: Tasks are stored in a contiguous, cache-friendly `GenerationalArena` rather than `HashMap`s or complex work-stealing queues, providing strictly $O(1)$ allocations and lookups.
*   **Bring-Your-Own I/O**: Volt avoids the heavy baggage of timers, file systems, or network reactors. It expects the host environment (like the browser's `fetch` API or `setTimeout`) to handle I/O, allowing Volt to focus entirely on raw scheduling throughput.

## 📊 Benchmarks

In fair, apples-to-apples benchmarking against industry standards, Volt scales exceptionally well due to its specialized constraints.

**Native Overhead (macOS, AArch64)**
When spawning 1,000,000 immediately-ready tasks, Volt is dramatically faster than general-purpose executors:
*   `pollster` (baseline, zero-scheduling block-on): 47.1 ms
*   **Volt**: 57.2 ms
*   `futures::executor::LocalPool`: 66.0 ms
*   `tokio` (`current_thread`): 211.6 ms
*   `async-executor` (`smol` local): 248.1 ms

*Volt achieves nearly 3.7x higher throughput than Tokio's current thread implementation.*

**WebAssembly (Node.js / V8)**
When compiled to WebAssembly and executed in V8, Volt successfully schedules 1,000,000 tasks in **105 milliseconds**, yielding a per-task runtime overhead of roughly **105 nanoseconds**.

## 📦 Getting Started

Add Volt to your `Cargo.toml`:
```toml
[dependencies]
volt = "0.1.0"
```

Initialize the runtime and spawn tasks:
```rust
use volt::Runtime;

fn main() {
    let mut rt = Runtime::new();

    // No `Send` bounds required!
    rt.spawn(async {
        println!("Hello from Volt!");
    });

    rt.run();
}
```

Check out the [README](https://github.com/iamfaiz123/volt/blob/main/README.md) for full documentation and production guidelines!
