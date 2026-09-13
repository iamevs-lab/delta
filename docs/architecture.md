# Architecture

## Separation

```
Neovim / CLI / future adapters
            |
            v
      delta-cli / RPC
            |
            v
      delta-runtime
       /    |     \
      v     v      v
  graph   store   domain crates (delta-text, later others)
      \     |     /
            v
        delta-core
```

`delta-core` has no graph, no scheduler, no Neovim, no lexer.
It defines change, identity, versions, fingerprints, and errors.

`delta-graph` does not know how to lex or parse. It knows nodes, edges, invalidation, and order.

`delta-runtime` asks computations for work. It does not hard-code text.

`delta-text` is the first domain crate. It is a client of the runtime, not its center.

## Cache vs Delta

A cache is a map: `input → result`.

Delta is a state machine:

- current state
- explicit dependencies
- versions
- incoming deltas
- invalidation
- reusable computation
- an oracle (full recompute) for correctness

If we only memoized function results, we would have a cache.
The interesting part is knowing *what changed* and *what that change touches*.

## Why not Salsa / Adapton / Incremental

Those systems are related prior art. Delta's job is not to reimplement them as a product.

Delta is a laboratory: every layer must be measurable, comparable against a full oracle, and willing to lose.

## Process model

The simplest reliable Neovim integration is a versioned Rust executable speaking JSON-lines over stdin/stdout.

Reasons:

- no native module ABI issues
- easy to rebuild during research
- same binary used by CLI replay and benches
- the editor is an adapter, not a host requirement

A future in-process embedding remains possible: the engine is a library.

## Persistence and parallelism

Designed, deferred.

- Persistence: `delta-store` exposes a `Store` trait. `MemoryStore` is the MVP.
- Parallelism: independent dirty nodes *may* run concurrently later. Today the scheduler is single-threaded and deterministic.
