# Experiment log

Each file is a research note, not a marketing page.

**Results stay `TBD` until a runner produces them.**

If you add a number by hand, you are breaking the project.

| File | Layer | Oracle |
|------|--------|--------|
| [001-text.md](001-text.md) | buffer apply | rebuild string from scratch |
| [002-lexer.md](002-lexer.md) | tokens | lex entire buffer |
| [003-parser.md](003-parser.md) | AST | parse entire token stream |
| [004-dependencies.md](004-dependencies.md) | symbols | rebuild def/ref table |
| [005-diagnostics.md](005-diagnostics.md) | diagnostics | recheck all nodes |
| [006-search.md](006-search.md) | search | scan entire buffer |
| [007-neovim.md](007-neovim.md) | adapter | full-buffer resend |
| [008-benchmark.md](008-benchmark.md) | harness | n/a |
| [009-runtime.md](009-runtime.md) | policy | AlwaysFull |
| [010-break-it.md](010-break-it.md) | adversarial | AlwaysFull |

Shared invariant:

```
incremental(state, delta) == full(apply(state, delta))
```
