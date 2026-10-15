// Cargo test runner for wasm32-wasip1: runs a test binary under Node's WASI,
// with the package directory (cargo's working directory) as "." inside.
import { readFile } from 'node:fs/promises'
import { WASI } from 'node:wasi'
import process from 'node:process'

const [, , wasmPath, ...args] = process.argv
const wasi = new WASI({
  version: 'preview1',
  args: [wasmPath, ...args],
  env: process.env,
  preopens: { '.': process.cwd() },
  returnOnExit: true,
})
const module = await WebAssembly.compile(await readFile(wasmPath))
const instance = await WebAssembly.instantiate(module, wasi.getImportObject())
process.exitCode = wasi.start(instance)
