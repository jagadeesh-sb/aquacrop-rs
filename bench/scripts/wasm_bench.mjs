// Times the aquacrop-rs WebAssembly build: node wasm_bench.mjs <pkg> <reps> <file.json>...
const T0 = performance.now()
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'
const [pkg, repsArg, ...files] = process.argv.slice(2)
const wasm = await import(pathToFileURL(join(pkg, 'aquacrop.js')).href)
wasm.initSync({ module: readFileSync(join(pkg, 'aquacrop_bg.wasm')) })
console.log(JSON.stringify({ import_ms: performance.now() - T0 }))
const reps = Math.max(1, Number(repsArg))
for (const f of files) {
  const doc = JSON.parse(readFileSync(f, 'utf8'))
  const ms = []
  let y = NaN
  for (let i = 0; i < reps; i++) {
    const t = performance.now()
    try {
      const res = JSON.parse(wasm.run(JSON.stringify(doc.input)))
      y = res.final_stats.reduce((a, s) => a + (s.dry_yield ?? 0), 0)
    } catch { /* AquaCrop-OSPy fails on these too */ }
    ms.push(performance.now() - t)
  }
  console.log(JSON.stringify({ file: f, ms, yield_sum: y }))
}
