# Changelog

All notable changes to Delta are recorded here.

The project follows [Semantic Versioning](https://semver.org/).
`0.x` is an experimental research preview. The API will change.

## [0.1.0-alpha] — unreleased

### Added

- Workspace skeleton: `delta-core`, `delta-graph`, `delta-store`, `delta-runtime`, `delta-text`, `delta-bench`, `delta-cli`
- Delta primitives: `Insert`, `Delete`, `Replace`, reserved `Move`, `CompositeDelta`
- Versioning, fingerprints, dependency graph, eager/lazy invalidation
- Runtime with full + incremental recomputation and explicit recompute policy
- Incremental text, lexer, parser, symbols, and diagnostics experiments
- Replay format and CLI (`inspect`, `graph`, `benchmark`, `replay`, `compare`, `stats`, `serve`)
- Neovim adapter that displays engine-produced metrics
- CI for Linux, macOS, and Windows
- Experiment log with `TBD` results until benches are executed
