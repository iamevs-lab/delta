# 003 — Incremental parser

## Hypothesis

If an edit is confined to one statement's tokens, other statement AST nodes can be reused with stable identities.

## Setup

Toy AST: program → statements → expressions.

```
A
+-- B
|   +-- C
|   +-- D
+-- E
    +-- F
    +-- G
```

Edit C. Expect C recomputed, B possibly updated, D/E/F/G reused if their token spans are untouched.

## Implementation

- Full: parse the whole token stream.
- Incremental: map dirty token range to statement index; reparse that statement; keep others.

## Workload

Many `let` statements; edit one identifier or one expression.

## Baseline

Full parse.

## Delta approach

Statement-granularity reuse.

## Measurements

`reused_nodes`, `invalidated_nodes`, `recomputed_nodes`, `affected_nodes`.

## Results

TBD

## Interpretation

TBD

## Failure cases

Edits that insert/delete statements shift later indices. We reassign identities for shifted statements rather than inventing fake stability.

## Conclusion

TBD

## Next question

Can symbols be updated from the dirty statement only?
