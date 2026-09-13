# 006 — Incremental search

## Hypothesis

A search index over a buffer can be updated from a text delta instead of scanning the whole file for every query or every edit.

## Setup

Not implemented in the MVP. The runtime interface is generic enough to register a search computation later.

## Implementation

TODO

## Workload

TODO

## Baseline

Full buffer scan.

## Delta approach

TODO — likely: adjust hit offsets after an edit, rescan only the dirty byte range.

## Measurements

TBD

## Results

TBD

## Interpretation

TBD

## Failure cases

Regexes that span the dirty range; case-folding; multi-buffer search.

## Conclusion

Deferred. Interface exists; experiment does not.

## Next question

Is search a good second domain after text, or should builds come first?
