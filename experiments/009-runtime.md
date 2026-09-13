# 009 — Runtime policy

## Hypothesis

A transparent heuristic (`affected_nodes / total_nodes` plus recent timings) can choose full recompute when incremental would lose, and we can show the decision.

## Setup

`RecomputePolicy::{AlwaysIncremental, AlwaysFull, Adaptive}`.

## Implementation

Adaptive compares estimated incremental cost vs estimated full cost. Both estimates and the actuals are on `UpdateReport`.

## Workload

Localized edits vs global-symbol edits vs huge paste.

## Baseline

AlwaysFull and AlwaysIncremental run on the same sequence.

## Delta approach

Adaptive.

## Measurements

`decision`, `estimated_incremental_cost`, `estimated_full_cost`, `actual_cost`.

## Results

TBD

## Interpretation

TBD

## Failure cases

Heuristics that oscillate; estimates that do not correlate. If that happens, document it — do not hide the decision.

## Conclusion

TBD

## Next question

Is graph-traversal cost first-class in the estimate, or still noise?
