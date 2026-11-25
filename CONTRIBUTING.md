# Contributing

Thank you for helping. Bug reports, questions and pull requests are welcome
through the issue tracker.

## Reporting a problem

Please include:

- the input JSON (or the smallest input that shows the problem);
- what you expected and what you got;
- whether AquaCrop-OSPy 3.1.0 gives the same result for the same input.

If AquaCrop-OSPy gives the same result, the behaviour belongs to the model
and should be raised with the AquaCrop-OSPy project as well. This port
follows AquaCrop-OSPy on purpose, including its less obvious behaviours.

## Changing the code

The port is meant to be read next to AquaCrop-OSPy. Please keep that true:

- Keep the Python module, function and variable names.
- Keep the order of arithmetic operations. A change in order can change the
  last bit of a result and break bit-for-bit agreement.
- Where AquaCrop-OSPy would fail, return an `Error` or take a defined branch,
  and mark the place with a `Deviation:` comment that says what Python does.
  Add it to [docs/differences.md](docs/differences.md).
- Model changes that AquaCrop-OSPy does not have belong upstream first.

Before opening a pull request:

```sh
pip install aquacrop==3.1.0 numpy==2.5.4 pandas==3.0.6
python scripts/golden.py
cargo test --release
cargo clippy --all-targets -- -D warnings
```

If you change anything the WebAssembly build uses, also build `pkg/` and run
`node scripts/wasm-check.mjs` (see [docs/webassembly.md](docs/webassembly.md)).

CI runs the same steps on every pull request.

## Following a new AquaCrop-OSPy release

1. Update the pinned version in `scripts/golden.py`, CI and `UPSTREAM_VERSION`
   in `src/lib.rs`.
2. Regenerate `tests/golden/` and run the tests.
3. Port each upstream change to the module of the same name, and note it in
   `CHANGELOG.md`.

## Licence

By contributing you agree that your contribution is licensed under the
Apache License, Version 2.0, the licence of this project.
