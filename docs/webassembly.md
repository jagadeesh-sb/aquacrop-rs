# Using aquacrop-rs in the browser

The crate builds to a WebAssembly module of 626 KB. It runs in the page with
no server, and nothing the user enters leaves the browser.

## Build

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.129   # same version as in Cargo.lock
cargo build --profile wasm --target wasm32-unknown-unknown --features wasm
wasm-bindgen --target web --out-dir pkg target/wasm32-unknown-unknown/wasm/aquacrop.wasm
```

`pkg/` then holds `aquacrop.js`, `aquacrop_bg.wasm` and TypeScript types.
Serve both files together.

## Functions

| Function | Returns |
|---|---|
| `run(input: string): string` | The `Results` as a JSON string. `input` is a `ModelInput` as a JSON string. Throws a JavaScript `Error` when the input cannot be simulated |
| `crop_names(): string` | The built-in crop names, as a JSON array |
| `crop_preset(name: string): string` | One built-in crop's parameters, as a JSON object, or `null` |
| `soil_types(): string` | The built-in soil types, as a JSON array |
| `version(): string` | This crate's version and the AquaCrop-OSPy version it follows |

Inputs and results are the same JSON as in Rust; see
[inputs.md](inputs.md) and [outputs.md](outputs.md).

## In a web page

```js
import init, { run, crop_names } from './pkg/aquacrop.js'

await init()                       // fetches and compiles aquacrop_bg.wasm
console.log(JSON.parse(crop_names()))

try {
  const results = JSON.parse(run(JSON.stringify(input)))
  for (const s of results.final_stats) {
    console.log(s.harvest_date, s.dry_yield.toFixed(2), 't/ha')
  }
} catch (e) {
  console.error('cannot simulate:', e.message)
}
```

A ten-year run takes tens of milliseconds (see
[performance.md](performance.md)), so it can run on the page's main thread.
For many runs at once, move the calls into a Web Worker.

## In Node.js

```js
import { readFileSync } from 'node:fs'
import { initSync, run } from './pkg/aquacrop.js'

initSync({ module: readFileSync('./pkg/aquacrop_bg.wasm') })
const results = JSON.parse(run(readFileSync('run.json', 'utf8')))
```

## Accuracy

The browser build matches AquaCrop-OSPy's season yields to within 0.00076%
on the reference scenarios. Daily values can differ in the last bit, and in a
few long runs a threshold (drainage, or the rounding of root-zone water) tips
one day earlier or later. See
[validation.md](validation.md).
