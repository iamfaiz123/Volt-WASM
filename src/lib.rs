//! # Volt — A lightweight, single-threaded async runtime
//!
//! Volt is a minimal async runtime built for learning and for environments
//! where a full multi-threaded runtime (like Tokio) is overkill — including
//! WASM targets.
//!
//! ## Quick Start
//!
//! ```rust
//! use volt::{Runtime, Delay};
//! use std::time::Duration;
//!
//! let mut rt = Runtime::new();
//! rt.spawn(async {
//!     println!("waiting 100ms…");
//!     Delay::new(Duration::from_millis(100)).await;
//!     println!("done!");
//! });
//! rt.run();
//! ```

mod arena;
mod combinator;
mod delay;
mod executor;
mod runtime;
mod task;
mod waker;

// ── Public re-exports ────────────────────────────────────────────────────────

pub use delay::Delay;
pub use runtime::Runtime;
pub use arena::TaskId;

/// Future combinators: [`join`], [`select`], [`map`], and the [`Either`] enum.
pub mod future {
    pub use crate::combinator::{join, map, select, Either, Join, Map, Select};
}
