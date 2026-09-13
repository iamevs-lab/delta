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

## Limitations

Honest ones, up front:

- Incremental is not always faster. High fanout, global symbols, and huge pastes can make incremental *worse*. That is a research result, not a failure to hide.
- Persistence across process restarts is designed but not required for the MVP. In-memory correctness comes first.
- Parallel recomputation is designed, not enabled. Execution is single-threaded and deterministic until correctness and benches exist.
- Fingerprints are optional and cost-accounted. They are not a magic cache key.
- The toy language used for lexer/parser experiments is not a real programming language.
- Neovim is an adapter. Installing the plugin without building the Rust CLI does nothing useful.

---



## Quick start



### Prerequisites

- Rust stable (1.85+; CI uses the latest stable)
- Neovim 0.9+ if you want the editor laboratory



### Build, test, benchmark

```bash
git clone https://github.com/iamevs-lab/delta.git
cd delta
cargo test --workspace
cargo run -p delta-cli -- --help
cargo run -p delta-cli -- benchmark --workload text-smoke --out benchmarks/results
cargo run --release -p delta-cli -- benchmark --workload text-smoke --out benchmarks/results


# build 
cargo build 

# Direct Install
./scripts/install-nvim.ps1 # for windows
./scripts/install-nvim.sh # for mac/linux
```

- For more runs refer `/scripts/*`


### Neovim (lazy.nvim)

`cargo build --release -p delta-cli` also copies the CLI and plugin to `~/.evs-delta` (no extra install step). Point lazy.nvim at that home path:

```lua
{
  name = "delta.nvim",
  dir = vim.fn.expand("~/.evs-delta/delta.nvim"),
  lazy = false,
  opts = {
    cmd = { vim.fn.expand("~/.evs-delta/bin/delta-cli.exe"), "serve" },
  },
}
```

On Unix the binary is `~/.evs-delta/bin/delta-cli`. Rebuild `delta-cli` after CLI or plugin changes.


---

## Experiments

Each experiment is a hypothesis with a baseline, a delta approach, and a place for measured results.


| #   | Question                                        | Log                                                                |
| --- | ----------------------------------------------- | ------------------------------------------------------------------ |
| 001 | Do local text edits avoid whole-buffer copies?  | [experiments/001-text.md](experiments/001-text.md)                 |
| 002 | Can a lexer retokenize only the dirty region?   | [experiments/002-lexer.md](experiments/002-lexer.md)               |
| 003 | Can an AST reuse nodes outside the edit?        | [experiments/003-parser.md](experiments/003-parser.md)             |
| 004 | Do symbol dependencies propagate correctly?     | [experiments/004-dependencies.md](experiments/004-dependencies.md) |
| 005 | Can diagnostics recompute only affected checks? | [experiments/005-diagnostics.md](experiments/005-diagnostics.md)   |
| 006 | Search as incremental computation               | [experiments/006-search.md](experiments/006-search.md)             |
| 007 | Neovim as a live laboratory                     | [experiments/007-neovim.md](experiments/007-neovim.md)             |
| 008 | Benchmark methodology                           | [experiments/008-benchmark.md](experiments/008-benchmark.md)       |
| 009 | Runtime policy and adaptive choice              | [experiments/009-runtime.md](experiments/009-runtime.md)           |
| 010 | When is Delta worse?                            | [experiments/010-break-it.md](experiments/010-break-it.md)         |


---


## Roadmap

Phased. A later phase does not start until the previous one is correct and tested.


| Phase | Focus                                | Status                               |
| ----- | ------------------------------------ | ------------------------------------ |
| 0     | Workspace, CI, docs, bench harness   | done                                 |
| 1     | Delta primitives                     | done                                 |
| 2     | State, versions, fingerprints        | done                                 |
| 3     | Dependency graph + invalidation      | done                                 |
| 4     | Runtime: full + incremental + policy | done                                 |
| 5     | Incremental text experiment          | done (oracle-tested)                 |
| 6     | Incremental lexer                    | done (falls back if ≠ full)          |
| 7     | Incremental parser                   | done (falls back if ≠ full)          |
| 8     | Symbols                              | done (incremental attempt + oracle)  |
| 9     | Diagnostics                          | done (incremental attempt + oracle)  |
| 10    | Neovim adapter                       | present; interactive measurement TBD |
| 11    | Adaptive recomputation               | heuristic present; unmeasured        |
| 12    | Replayable experiments               | CLI present                          |
| 13    | Advanced / adversarial benches       | workloads present; most sizes TBD    |
| 14    | Persistence layer                    | later                                |
| 15    | Parallel scheduler                   | later                                |


---



## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

The useful contributions are: correctness tests, measured benchmarks, failure cases, and smaller APIs.
The useless contributions are: unmeasured speed claims.

---



## License

MIT. See [LICENSE](LICENSE).
