# 010 — Break it

## Hypothesis

Delta is worse than full recomputation under at least one of:

- one global symbol referenced everywhere
- invalidation fanout ≈ all nodes
- deep dependency chains where traversal dominates
- alternating edits that defeat resync
- huge paste / huge delete

## Setup

Adversarial workloads in `delta-bench`.

## Implementation

Same engine. No special "make it look good" path.

## Workload

`global_symbol`, `massive_fanout`, `deep_nest`, `alternating`, `huge_paste`, `huge_delete`.

## Baseline

AlwaysFull.

## Delta approach

AlwaysIncremental (forced, so we can see the loss).

## Measurements

When incremental latency > full latency; overhead breakdown (invalidation vs hash vs compute).

## Results

`massive_fanout` (debug, 4100 bytes, rename `foo` → `bar` with many call sites):

- incremental tokens: 2341 reused, 1 recomputed
- incremental AST: 584 reused, 1 recomputed
- latency: incremental 4.44 ms vs full 1.35 ms (`incremental_faster: false`)
- Δ/N bytes: 0.67
- correctness: pass

So even when almost all tokens were reused, incremental was slower in this debug run.
Larger N, release builds, and whether Adaptive would flip to full: TBD.

## Interpretation

Fanout of *invalidation* was not the whole story here: the lexer reused the suffix.
The loss is overhead (graph/pipeline bookkeeping, extra passes, debug codegen) relative to a cheap full walk of 4 KiB.

This is an answer to "when is Delta worse?": **when N is small and incremental machinery costs more than scanning N.**
The engine can already expose `incremental_faster` and Δ/N; Adaptive should eventually use that.

## Failure cases

This experiment *is* the failure case.

## Conclusion

TBD. The useful sentence is: **when, why, which overhead, and can Adaptive detect it?**

## Next question

What is the smallest signal Adaptive needs to flip to full?
