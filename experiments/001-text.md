# 001 — Incremental text

## Hypothesis

Local edits can update a buffer without processing the entire contents. The interesting metric is **bytes touched**, not a slogan about complexity.

## Setup

`delta-text::TextBuffer` plus the full-rebuild oracle (`String` rebuild).

## Implementation

- Full: allocate a new string and copy the whole result.
- Incremental: apply `Insert` / `Delete` / `Replace` to the existing buffer (std `String` splice). This is still a memmove of the tail; we measure it instead of pretending it is O(1).

## Workload

Sizes: 1 KiB, 10 KiB, 100 KiB, 1 MiB, 10 MiB.
Edits: 1 char / 10 chars / 1 line / 100 lines / random / begin / middle / end.

## Baseline

Full string rebuild after every edit.

## Delta approach

Apply the delta in place; record `bytes_processed` as the moved tail plus the inserted bytes.

## Measurements

See `delta-cli benchmark --workload text-*`.

## Results

TBD

## Interpretation

TBD

## Failure cases

Huge insert/delete near the beginning still moves almost the whole buffer. A piece table / rope is a later experiment, not this one.

## Conclusion

TBD

## Next question

When does a fancier representation beat `String` splice, after accounting for lookup cost?
