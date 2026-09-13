# Security

Delta treats input as untrusted.

The engine accepts text, edit streams, serialized replay files, and (via Neovim) buffer contents. Those are data, not trusted code.

## What this project is

An experimental in-process computation runtime. It is not a sandbox. It is not a multi-tenant service.

## What to watch for

- Uncontrolled graph growth (node / edge limits exist; do not remove them without a replacement)
- Pathological dependency depth (invalidation and scheduling are iterative, not unbounded recursive)
- Replay files that claim huge buffers or unbounded composite deltas
- Neovim feeding the engine unsanitized buffer text (expected; still must not crash the process)

## Reporting

If you find a crash, unbounded allocation, or a way to make the engine loop on valid-looking input, open a private report with the repository owner or file an issue titled `security:` if the project has no private channel yet.

Please include:

- engine version / commit
- a replay file if possible (`delta replay`)
- memory / time observations

Do not include secrets from other systems. Delta does not need them.

## Supported versions

`0.1.x` is an experimental research preview. Security fixes land on the default branch.
