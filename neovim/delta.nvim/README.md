# delta.nvim

Neovim adapter for Delta. The editor is a laboratory, not the product.

The plugin does **not** reimplement incremental analysis. It:

1. Turns buffer byte changes into `Delta` values (`nvim_buf_attach` / `on_bytes`)
2. Sends them to `delta-cli serve`
3. Displays fields that came back from the engine / pipeline

If a number cannot be measured, the UI shows `n/a`.

## Install (recommended)

From the repo:

```bash
./scripts/install-nvim.ps1 #if windows
./scripts/install-nvim.sh #if mac/linux
```

Debug builds (`cargo build -p delta-cli`) update the same paths. Neovim does not need the repo after this. Set `DELTA_SKIP_NVIM_INSTALL=1` to skip the copy.

## lazy.nvim

```lua
{
  name = "delta.nvim",
  dir = vim.fn.expand("~/.evs-delta/delta.nvim"),
  lazy = false,
  opts = {
    cmd = { vim.fn.expand("~/.evs-delta/bin/delta-cli.exe"), "serve" },
    -- Unix:
    -- cmd = { vim.fn.expand("~/.evs-delta/bin/delta-cli"), "serve" },
  },
}
```

`~` is the user profile (`C:/Users/you` on Windows). You can omit `cmd`; the plugin also looks in `~/.evs-delta/bin/` on its own.

Rebuild `delta-cli` after you change the Rust CLI or the Lua plugin.

## Development (repo checkout)

If you are hacking on the plugin in-tree, you can still point `dir` at `neovim/delta.nvim` and skip the copy. The plugin then searches, in order:

1. `opts.cmd`
2. `~/.evs-delta/bin/delta-cli`
3. `target/release` or `target/debug` next to this repo
4. `delta-cli` on `PATH`
5. `cargo run -p delta-cli -- serve`

## Commands

| Command | Action |
|---------|--------|
| `:DeltaStatus` | Last **engine** update (not zeros) |
| `:DeltaInspect` | Current pipeline |
| `:DeltaGraph` | ASCII graph + node states |
| `:DeltaStats` | Graph statistics |
| `:DeltaBenchmark` | How to run real benches |
| `:DeltaReset` | Re-open the current buffer |
| `:DeltaToggle` | Enable / disable attach |

`vim.g.delta_statusline` is updated after each real edit. You can put it on your statusline:

```lua
vim.o.statusline = vim.o.statusline .. "  %{get(g:,'delta_statusline','')}"
```

## Events

`BufRead` / `BufEnter` open a per-buffer session. `on_bytes` sends incremental edits.
Closing a Delta float does **not** reset the engine. `BufDelete` on a tracked source buffer sends `close` for that buffer only.
