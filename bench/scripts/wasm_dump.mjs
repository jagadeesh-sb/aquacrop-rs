// Writes the WebAssembly build's full results: node wasm_dump.mjs <pkg> <out_dir> <file.json>...
import { readFileSync, writeFileSync } from 'node:fs'
import { join, basename } from 'node:path'
import { pathToFileURL } from 'node:url'
const [pkg, out, ...files] = process.argv.slice(2)
const wasm = await import(pathToFileURL(join(pkg, 'aquacrop.js')).href)
wasm.initSync({ module: readFileSync(join(pkg, 'aquacrop_bg.wasm')) })
for (const f of files) {
  const doc = JSON.parse(readFileSync(f, 'utf8'))
  let body
  try { body = wasm.run(JSON.stringify(doc.input)) } catch (e) { body = JSON.stringify({ error: String(e.message ?? e) }) }
  writeFileSync(join(out, basename(f)), body)
}
