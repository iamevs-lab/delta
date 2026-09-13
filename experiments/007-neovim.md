# 007 — Neovim laboratory

## Hypothesis

`nvim_buf_attach` `on_bytes` can produce text deltas without sending the whole buffer on every keystroke. The status UI can show engine metrics from the last `UpdateReport`.

## Setup

`neovim/delta.nvim` + `delta-cli serve` (JSON-lines).

## Implementation

- Adapter translates `on_bytes` → `TextDelta`.
- Engine runs the text/lexer/parser pipeline.
- UI reads `UpdateReport` fields only.

## Workload

Interactive typing; paste; undo (undo arrives as another byte change).

## Baseline

Send entire buffer on `TextChanged` / `TextChangedI`.

## Delta approach

Byte-range deltas.

## Measurements

Bytes sent over RPC, update latency, UI field provenance (must be engine fields).

## Results

TBD

## Interpretation

TBD

## Failure cases

Binary-ish buffers, very large pastes, detach on reload. If incremental bytes are unavailable, the adapter may send a full replace and must say so.

## Conclusion

TBD

## Next question

Does the visual loop change how we design metrics?
