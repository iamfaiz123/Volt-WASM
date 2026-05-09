const { Runtime } = require('../pkg/volt.js');

function time_volt(n) {
    const t0 = performance.now();
    const rt = Runtime.new();
    
    // Create an empty JS Promise resolving immediately to simulate a task
    // Note: since our runtime takes `F: Future`, we can't easily pass JS
    // objects into the pure Rust `spawn` via wasm-bindgen without adding
    // explicitly exported JS wrappers for spawning.
    
    // So this proves `wasm-pack test` is actually what we need...
    // Let me figure out why the logs were completely swallowed.
}
