# 008 — Benchmark laboratory

## Hypothesis

A small custom harness (not a opaque benchmark crate) is enough to compare full vs incremental honestly and emit replayable JSON.

## Setup

`delta-bench` + `delta-cli benchmark`.

## Implementation

Deterministic workload generators with an explicit RNG seed.

## Workload

See `docs/benchmarks.md`.

## Baseline

`AlwaysFull`.

## Delta approach

`AlwaysIncremental` and later `Adaptive`.

## Measurements

Latency, bytes, node reuse, policy decision.

## Results

First measured run (this repository, **debug** profile, Windows, 2026-09-12).
Do not treat these as release-performance claims.

| workload | size | full ms | inc ms | inc faster? | Δ/N bytes | tokens reused/recomputed |
|----------|------|---------|--------|-------------|-----------|--------------------------|
| text-smoke | 2052 | 0.88 | 1.99 | no | 0.40 | 709 / 1 |
| single_character | 4100 | 1.15 | 3.30 | no | 0.67 | 1349 / 1 |
| massive_fanout | 4100 | 1.35 | 4.44 | no | 0.67 | 2341 / 1 |

All three runs: `correct: true` (incremental output matched the full oracle).

JSON: `benchmarks/results/` after a local `delta-cli benchmark`.

Release-profile and larger sizes: TBD.

## Interpretation

On ~2–4 KiB inputs in a **debug** build, incremental did much less *work* (bytes and tokens) and still lost on *latency*.
That is consistent with bookkeeping overhead dominating when N is small and the compiler is unoptimized.

This is a useful result: **less work ≠ less time** until N and the optimizer catch up.
Confirm or refute under `--release` before saying anything stronger.

## Failure cases

Noisy laptops, first-run page faults, debug builds. Record `profile` and `opt_level` in the JSON.

## Conclusion

TBD

## Next question

Where is the Δ/N crossover on this machine?
