# Cache vs Delta

## Cache

```
key(input) → stored result
```

A cache answers: *have I seen this input?*

It does not know:

- which part of the input changed
- which downstream computations that part feeds
- whether two different inputs share a still-valid subgraph

## Delta

```
state
  + dependency relationships
  + versions
  + incoming delta
  + invalidation
  + reusable computation
```

Delta answers: *what changed, what depends on that, and what can remain valid?*

Fingerprints may look like cache keys. They are optional evidence, cost-accounted, and never a substitute for the graph.

## Why the distinction matters

If we reduce this project to memoization, the Neovim UI will show "cache hits" and we will have learned little.

The laboratory is about **impact**.
A one-character edit that invalidates one token and one AST node is a different phenomenon than a cache miss on the whole file hash.
