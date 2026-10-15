// Run every reference scenario through the WebAssembly build and compare
// with AquaCrop-OSPy.
//   node scripts/wasm-check.mjs [pkg-dir] [golden-dir]
//
// The browser's maths library (Rust's libm) differs from glibc's in the last
// bit of exp, log and pow, so daily values are not bit-identical and, in a
// few long runs, a threshold tips a day earlier or later. The check therefore
// reports daily agreement and fails only when a season result (dry yield,
// potential yield, irrigation) differs by more than SEASON_RTOL.
import { readFileSync, readdirSync } from 'node:fs'
import { join, resolve } from 'node:path'
import { pathToFileURL } from 'node:url'

const pkg = resolve(process.argv[2] ?? 'pkg')
const goldenDir = resolve(process.argv[3] ?? 'tests/golden')
const wasm = await import(pathToFileURL(join(pkg, 'aquacrop.js')).href)
wasm.initSync({ module: readFileSync(join(pkg, 'aquacrop_bg.wasm')) })
console.log(wasm.version())

const RTOL = 1e-9, ATOL = 1e-9, SEASON_RTOL = 1e-4
// The JSON output writes non-finite numbers as null.
const num = (v) => (v === null || v === 'inf' || v === '-inf' ? NaN : v)
const close = (a, b) => a === b || (Number.isNaN(a) && Number.isNaN(b)) || Math.abs(a - b) <= ATOL + RTOL * Math.abs(b)

let dailyOk = 0, seasonOk = 0, fail = 0, worstSeason = 0, totalMs = 0
for (const f of readdirSync(goldenDir).filter((f) => f.endsWith('.json')).sort()) {
  const doc = JSON.parse(readFileSync(join(goldenDir, f), 'utf8'))
  const t0 = performance.now()
  let res, err
  try {
    res = JSON.parse(wasm.run(JSON.stringify(doc.input)))
  } catch (e) {
    err = String(e.message ?? e)
  }
  totalMs += performance.now() - t0
  if (doc.error) {
    if (err || !doc.error.startsWith('AssertionError')) dailyOk++
    else { fail++; console.log(`FAIL ${doc.name}: runs, AquaCrop-OSPy fails (${doc.error})`) }
    continue
  }
  if (err) { fail++; console.log(`FAIL ${doc.name}: ${err}`); continue }
  let daily = 0
  for (const t of ['water_flux', 'water_storage', 'crop_growth']) {
    const want = doc.expected[t], got = res[t]
    want.columns.forEach((c, ci) => {
      const g = got.values[got.columns.indexOf(c)].map(num), w = want.values[ci].map(num)
      for (let d = 0; d < w.length; d++) if (!close(g[d], w[d])) daily++
    })
  }
  const fs = doc.expected.final_stats
  let worst = fs.length === res.final_stats.length ? 0 : Infinity
  fs.forEach((w, i) => {
    const g = res.final_stats[i]
    if (!g || g.harvest_date !== w.harvest_date) { worst = Infinity; return }
    for (const k of ['dry_yield', 'yield_potential', 'seasonal_irrigation']) {
      const a = g[k] ?? NaN, b = w[k] ?? NaN
      if (!close(a, b)) worst = Math.max(worst, Math.abs(a - b) / Math.max(Math.abs(b), 1e-6))
    }
  })
  worstSeason = Math.max(worstSeason, worst)
  if (worst > SEASON_RTOL) { fail++; console.log(`FAIL ${doc.name}: season results differ by ${worst.toExponential(2)}`) }
  else if (daily) { seasonOk++; console.log(`near ${doc.name}: ${daily} daily values beyond 1e-9; seasons within ${worst.toExponential(1)}`) }
  else dailyOk++
}
console.log(`${dailyOk} match day by day, ${seasonOk} match season by season, ${fail} fail; ` +
  `worst season difference ${worstSeason.toExponential(2)}; ${totalMs.toFixed(0)} ms for all runs`)
process.exitCode = fail ? 1 : 0
