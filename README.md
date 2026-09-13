# iamevs-lab/delta

> An experimental incremental computation engine for software that changes continuously.

The goal is not to make computation faster.
The goal is to make unnecessary computation disappear.

**Status:** experimental research preview (`0.1.0-alpha`).
Do not treat this as a stable API or a production incremental compiler.

---


## Philosophy

Most software treats change as replacement.

A file changed → parse again.
A dependency changed → rebuild again.
A query changed → run again.

The previous result is discarded. The system starts over.

Delta explores a different primitive:

```
STATE₀
   |
   | Δ
   v
STATE₁
```

Instead of `f(input) → output`, the core question is:

```
f(state, previous_state, delta) → delta_output
```

A cache knows: *I've seen this before.*

Delta knows: *I know what changed, and what depends on it.*

---



## Problem

Continuously changing systems spend most of their time recomputing work that is still valid.

Typical pipelines look like this:

```
source → tokens → AST → symbols → diagnostics → references
```

A one-character edit is a tiny delta. A full pipeline is not. The research question is:

> How much computation can safely disappear when a system understands
> the relationship between state changes and computational dependencies?

Two questions must be answered before any recomputation:

1. **Change detection** — what actually changed?
2. **Impact analysis** — what computation is actually affected?

Only then should the engine recompute, and only the affected subgraph.

---



## Thesis

```
CHANGE
   |
   v
UNDERSTAND
   |
   v
PROPAGATE
   |
   v
RECOMPUTE ONLY WHAT MATTERS
   |
   v
ΔOUTPUT
```

The engine must always have a fallback. Incremental reasoning is a hypothesis, not a guarantee.

If proving what is unchanged costs more than recomputing, the engine should choose full recomputation and say so.

Every performance claim in this repository must come from a benchmark.
Unmeasured claims are bugs in the documentation.

---



## Architecture

The Rust core does not depend on Neovim.

Neovim is the first laboratory — an adapter that turns editor edits into deltas and renders real engine metrics.

```
crates/
  delta-core      delta model, versions, fingerprints, errors
  delta-graph     dependency graph, invalidation, scheduling order
  delta-store     in-memory node store (persistence is a later layer)
  delta-runtime   engine, policies, scheduler, metrics
  delta-text      first domain: text, lexer, parser, symbols, diagnostics
  delta-bench     reproducible workloads and comparison harness
  delta-cli       inspect / graph / replay / benchmark / nvim-serve

neovim/delta.nvim   editor adapter and status UI
```

Future adapters (VS Code, LSP, CLI build tools, library embedding) should sit beside Neovim, not inside the core.

---



## How it works



### Delta model

A delta is a transformation:

```
old_state + delta = new_state
```

The first implementation is text (`Insert`, `Delete`, `Replace`, reserved `Move`, `CompositeDelta`).
The engine is not hard-coded to source code. Text is experiment one, not the type system.

### Dependency graph

Every computation node records upstream dependencies and downstream dependents.

```
SOURCE → TOKENS → AST → SYMBOLS
                      → DIAGNOSTICS
                      → STRUCTURE
         SYMBOLS → REFERENCES
```

The graph supports creation, deletion, edge updates, traversal, invalidation, topological scheduling, and cycle detection.

### Invalidation

When a source changes, Delta does not recompute everything.

```
delta arrives
    → detect affected nodes
    → invalidate directly affected nodes
    → propagate invalidation
    → construct affected subgraph
    → schedule recomputation
    → reuse everything else
```

Eager and lazy invalidation are both implemented.

### Incremental computation

The runtime supports:

- full recomputation (the correctness oracle)
- incremental recomputation (the experiment)
- an explicit `RecomputePolicy`: `AlwaysIncremental`, `AlwaysFull`, `Adaptive`

Adaptive uses measured heuristics (affected ratio, recent timings, traversal cost).
It is not an ML system. Decisions are exposed: estimated cost, chosen strategy, actual cost.

### Versioning and fingerprints

Nodes carry versions and optional input/output fingerprints.

If `H(new_input) == H(previous_input)`, reuse is allowed.
Hashing has a cost. The engine measures hashing time so we can study when proving "unchanged" is more expensive than recomputing.

---

