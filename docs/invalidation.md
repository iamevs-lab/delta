# Invalidation

## Question

When a delta arrives at a node, which other nodes must stop trusting their output?

## Strategies

### Eager

On apply, walk dependents immediately and mark them `Dirty` with an `InvalidationReason`.

- Pro: the dirty set is known before scheduling; metrics are complete; adaptive policy can see fanout now.
- Con: pays traversal cost even if the user never reads those nodes.

### Lazy

Mark only the changed node. Dependents discover staleness when queried by comparing dependency versions.

- Pro: cheap writes; unused subgraphs stay untouched.
- Con: query path does more work; fanout is hidden until read.

The engine default is **eager** for the laboratory: we want to *see* impact.
Lazy is available via `InvalidationMode::Lazy`.

## Propagation

Eager invalidation is iterative (worklist), not recursive:

1. seed = directly affected nodes
2. while worklist: mark node dirty, enqueue unmarked dependents
3. stop if `max_invalidation_nodes` is exceeded → error, not a silent hang

Cycles are detected at edge-registration time. A cycle is a graph error, not an invalidation loop.

## What is *not* invalidated

Nodes that do not transitively depend on the changed node keep their version and output.
That is the entire point.

## Tradeoff we will measure

Invalidation itself has a cost. For tiny graphs it is noise.
For a highly connected symbol, invalidation fanout can exceed recompute of a small artifact.

See experiment 010.
