# 005 — Incremental diagnostics

## Hypothesis

Diagnostics whose dependencies are clean can be reused. A change in one function should not recompute diagnostics for unrelated functions.

## Setup

Pipeline: AST → syntax diagnostics, symbol diagnostics, reference diagnostics.

## Implementation

- Full: run all checkers.
- Incremental: run checkers only for dirty AST / symbol nodes.

## Workload

Undeclared identifier in one statement; unused binding in another; edit a third clean statement.

## Baseline

Full diagnostic pass.

## Delta approach

Checker-per-dependency.

## Measurements

`diagnostics_before`, `diagnostics_after`, `diagnostics_reused`, `diagnostics_recomputed`.

## Results

TBD

## Interpretation

TBD

## Failure cases

A missing `}`-style error in a real language would fan out. The toy parser is statement-oriented; we document that limit rather than fake recovery.

## Conclusion

TBD

## Next question

Can the Neovim UI display these counters without a second computation?
