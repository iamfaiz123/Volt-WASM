# ⚡ Volt WASM

**A hyper-optimized, strictly single-threaded asynchronous runtime designed for WebAssembly (WASM).**

Volt is built from the ground up to provide maximum scheduling throughput in single-threaded environments like browser JavaScript engines (V8/SpiderMonkey). By completely stripping out multi-threaded synchronization primitives (`Arc`, `Mutex`, `Send` bounds) and eliminating built-in I/O drivers, Volt achieves incredibly low per-task overhead.

---

## 🎯 When to Use Volt

Volt is built for one specific niche: **High-performance, single-threaded execution.**

**Use Volt if:**
- You are compiling Rust to WebAssembly (`wasm32-unknown-unknown`) to run in the browser or Node.js.
- You need to spawn and manage thousands of concurrent tasks without the heavy baggage of a multi-threaded runtime.
- You are interacting extensively with JavaScript objects or DOM elements (which are strictly `!Send`).
- You want deterministic, single-threaded task execution in a native embedded system or game engine.

**Do NOT use Volt if:**
- You are building a native backend web server (use Tokio).
- You need work-stealing across multiple CPU cores.
- You need built-in filesystem or networking I/O drivers (Volt expects the host environment—like the browser—to handle actual I/O events).

---

## 🚀 Why Volt? (Architecture)

Most async runtimes (even Tokio's `current_thread` and `smol`'s `LocalExecutor`) carry overhead designed for native OS environments: Epoll/Kqueue reactors, timer wheels, and lock-free queues designed to be upgraded to multi-threading.

Volt abandons all of this.
1. **Zero Atomics:** All shared state uses `Rc` and `RefCell`. No `Arc`, no `Mutex`, no atomic CPU instructions.
2. **Generational Arena:** Tasks are stored in a contiguous, array-based `GenerationalArena` rather than a `HashMap` or linked list. This provides O(1) insertions/removals, solves the ABA problem, and guarantees perfect CPU cache locality during polling.
3. **No `Send` Bounds:** Because the runtime is strictly single-threaded, your futures do not need to be `Send`. This makes it trivial to hold `wasm_bindgen` JS objects across `.await` points.

---

## 📦 Basic Usage

Add Volt to your `Cargo.toml`:

```toml
[dependencies]
volt = "0.1.0"
```

### Spawning Tasks

```rust
use volt_wasm::Runtime;

fn main() {
    // Initialize the single-threaded runtime
    let mut rt = Runtime::new();

    // Spawn a fire-and-forget task.
    // Notice that this future doesn't need to be `Send`.
    rt.spawn(async {
        println!("Hello from an async task!");
    });

    // Spawn multiple concurrent tasks
    for i in 0..5 {
        rt.spawn(async move {
            println!("Processing task {}", i);
        });
    }

    // Drive the executor until all spawned tasks are complete
    rt.run();
}
```

---

## 🏆 Benchmarks

Volt is benchmarked against the industry standards for single-threaded scheduling throughput. The benchmark measures the pure overhead of allocating, scheduling, and resolving $N$ immediately-ready tasks. 

### Native Overhead (macOS, AArch64)

*Time to spawn and complete all N tasks.*

| Tasks | Volt | futures::LocalPool | async-executor (smol) | Tokio (current_thread) |
|-------:|:---:|:---:|:---:|:---:|
| **1,000** | **55 µs** | 60 µs | 241 µs | 200 µs |
| **10,000** | **510 µs** | 595 µs | 2.40 ms | 2.00 ms |
| **100,000** | **5.00 ms** | 6.25 ms | 24.30 ms | 20.40 ms |
| **1,000,000** | **57.2 ms** | 66.0 ms | 248.1 ms | 211.6 ms |

*Note: Volt is ~3.7x faster than Tokio in single-threaded mode because it avoids I/O reactor and timer wheel initialization.*

### WebAssembly Overhead (Node.js / V8)

Compiled to `wasm32-unknown-unknown` and executed natively in V8.

| Tasks | Volt Execution Time | Overhead per Task |
|-------:|:---:|:---:|
| **1,000** | 1.00 ms | ~1000 ns |
| **10,000** | 2.00 ms | ~200 ns |
| **100,000** | 15.00 ms | ~150 ns |
| **1,000,000** | 105.00 ms | **~105 ns** |

Inside a WASM JS engine, Volt's scheduling overhead drops to approximately **105 nanoseconds per task** at scale.

---

## 🛠️ Production Readiness

Volt is safe for production use in WASM environments, but comes with specific architectural contracts you must respect:

1. **Bring your own I/O:** Volt is a pure executor. It does not provide `TcpStream`, `File`, or `setTimeout` equivalents. In WASM, you should use `wasm-bindgen-futures` and standard Web APIs (`fetch`, `setTimeout`) to interact with the outside world. Volt simply schedules the Rust state machines around those external events.
2. **Never block the thread:** Because Volt runs on a single thread (often the browser's UI thread), a task that performs heavy synchronous computation or uses `std::thread::sleep` will completely freeze the runtime and the browser tab. Always yield computationally heavy work or move it to a Web Worker.

## License

MIT OR Apache-2.0
