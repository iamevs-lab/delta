# 002 — Incremental lexer

## Hypothesis

A local identifier edit can be retokenized from a nearby token start to a resync point, preserving a suffix of the token stream and stable identities for untouched tokens.

## Setup

Toy language statements: `let name = expr;` plus `fn` / call sites.

## Implementation

- Full: scan `[0, len)`.
- Incremental: find the token covering the edit, rescan, resync when kind+lexeme match the old stream.

## Workload

Same size ladder as 001. Edits: `foo` → `foobar`, insert space, break a token, edit at EOF.

## Baseline

Naive lexer over the whole buffer.

## Delta approach

Dirty-region retokenize + suffix reuse.

## Measurements

Bytes scanned, tokens reused / recomputed, latency.

## Results

TBD

## Interpretation

TBD

## Failure cases

Edits that change later token boundaries (inserting `"` or removing a newline in some languages). The toy lexer has no strings; we still test identifier/number/operator breakage.

## Conclusion

TBD

## Next question

How much of the AST can survive a token-stable suffix?
