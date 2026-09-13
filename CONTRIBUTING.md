# Contributing to EVS Delta

This is a systems research repository. The useful unit of work is a measured experiment, not a larger feature surface.

## Ground rules

1. Do not add unsupported performance claims.
2. Incremental results must match the full-computation oracle.
3. If a benchmark was not run, write `TBD`. Do not invent numbers.
4. If a feature is incomplete, mark it `TODO` and keep the fallback (full recomputation).
5. The Rust core must not depend on Neovim.

## Development workflow

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p delta-cli -- benchmark --workload text-smoke --out benchmarks/results
```

On Windows PowerShell the same commands work.

## Commit messages

Use small, meaningful commits:

```
feat(core): add delta primitives
feat(graph): add dependency tracking
feat(runtime): add incremental scheduler
test(core): add delta property tests
bench(lexer): add incremental lexer benchmark
feat(nvim): add delta status
docs: document invalidation model
```

## Adding a computation

1. Implement a full oracle.
2. Implement incremental (or return `None` to fall back).
3. Add a differential test: `incremental(state, Δ) == full(apply(state, Δ))`.
4. Add a workload that can make incremental *lose*.
5. Log the experiment. Results stay `TBD` until executed.

## API changes

This crate is an experimental research preview. Breaking changes are allowed in `0.x`, but they must be explained in `CHANGELOG.md`.

## What we will reject

- Fake metrics or hard-coded benchmark values
- Dependencies without a reason
- Core changes that couple the engine to an editor
- "Always faster" language in docs or UI
