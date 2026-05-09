console.log(require('child_process').execSync('WASM_BINDGEN_TEST_NO_CAPTURE=1 node ./target/wasm32-unknown-unknown/release/deps/wasm_bench-5b71b231f3656ac5.wasm', {encoding: 'utf-8'}))
