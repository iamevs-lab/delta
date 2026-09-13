# Benchmark methodology

## Rule

Never optimize from intuition. Never publish a number that was not produced by a runner.

## Comparison

Every workload runs **both**:

1. `AlwaysFull` — discard incremental state, recompute from the new snapshot
2. The policy under test (`AlwaysIncremental` or `Adaptive`)

Same initial text. Same delta sequence. Same seed.

Correctness gate: incremental output == full output, or the run is a failure, not a win.

## Metrics (engine-produced)

- latency (invalidation, hashing, compute, total)
- bytes processed
- allocations (where `delta_alloc` counting is enabled)
- reused / invalidated / recomputed nodes
- affected graph percentage
- policy decision + estimated vs actual cost

CPU percent is recorded only when the host exposes it; otherwise `n/a`.

## Workloads

Sizes: 1 KiB, 10 KiB, 100 KiB, 1 MiB, 10 MiB (100 MiB where memory allows).

Edits: 1 char, 10 chars, 1 line, 100 lines, random region, begin / middle / end.

Patterns: `single_character`, `single_token`, `single_line`, `function_body`, `function_signature`, `import`, `global_symbol`, `rename`, `large_insert`, `large_delete`, `random_edit`.

Adversarial: one global symbol, massive fanout, deep nesting, huge paste/delete.

## Output

Machine-readable: `benchmarks/results/<name>-<timestamp>.json`

Human-readable: `delta-cli benchmark` prints a table. It does not invent missing fields.

## Replay

Every JSON result embeds enough to replay:

```
initial_state + [delta_1, delta_2, ...]
```

```
delta-cli replay path/to/file.json
delta-cli compare --full --incremental path/to/file.json
```

## Crossover

We study `Δ / N`:

- `Δ` = measured incremental work (bytes or nodes)
- `N` = measured full work

The interesting result is the region where incremental latency exceeds full latency.
That crossover is a feature of the research, not a secret.
