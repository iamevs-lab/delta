# 004 — Symbol dependencies

## Hypothesis

Changing a definition invalidates its reference sites and not unrelated symbols. Changing an unreferenced local invalidates almost nothing downstream.

## Setup

```
fn foo() {}
foo();
foo();
foo();
```

Graph: `foo_definition → {call_site_1, call_site_2, call_site_3}`.

## Implementation

- Full: rebuild the symbol table from the AST.
- Incremental: update defs/refs for dirty statements; retarget edges.

## Workload

Local rename vs renaming a highly connected symbol.

## Baseline

Full symbol extraction.

## Delta approach

Dependency-aware update.

## Measurements

Invalidation fanout, recomputed refs, reused refs.

## Results

TBD

## Interpretation

TBD

## Failure cases

One global name used everywhere: incremental invalidation ≈ full rebuild, plus graph overhead.

## Conclusion

TBD

## Next question

Do diagnostics follow the same edges?
