# Roadmap

Phases are sequential. A phase is done when it builds, tests, and does not lie.

| Phase | Content |
|------|---------|
| 0 | Workspace, CI, docs, experiment logs, bench harness |
| 1 | `Insert` / `Delete` / `Replace` / `CompositeDelta` |
| 2 | Node identity, versions, fingerprints |
| 3 | Graph, invalidation, traversal, cycles |
| 4 | Engine, full + incremental, `RecomputePolicy` |
| 5 | Incremental text + benches |
| 6 | Incremental lexer + benches |
| 7 | Incremental parser + benches |
| 8 | Symbols |
| 9 | Diagnostics |
| 10 | Neovim adapter, status, inspect, graph |
| 11 | Adaptive policy with visible decisions |
| 12 | Replay |
| 13 | Adversarial benches, crossover |
| 14 | Persistence (optional layer) |
| 15 | Parallel scheduler |

Deferred on purpose: persistence, parallelism, Move application, non-text domains, VS Code / LSP adapters.
