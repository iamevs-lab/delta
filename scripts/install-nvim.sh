#!/usr/bin/env bash
# Thin wrapper: cargo build copies the CLI + plugin to ~/.evs-delta/
set -euo pipefail

REPO="$(cd "$(dirname "$0")/.." && pwd)"
HOME_DELTA="${HOME}/.evs-delta"

echo "Building release delta-cli (installs to ${HOME_DELTA})..."
(cd "$REPO" && cargo build --release -p delta-cli && cargo run --release -p delta-cli -- --version >/dev/null)

echo
echo "Installed:"
echo "  CLI    ${HOME_DELTA}/bin/delta-cli"
echo "  plugin ${HOME_DELTA}/delta.nvim"
echo
cat <<'EOF'
lazy.nvim:
{
  name = "delta.nvim",
  dir = vim.fn.expand("~/.evs-delta/delta.nvim"),
  lazy = false,
  opts = {
    cmd = { vim.fn.expand("~/.evs-delta/bin/delta-cli"), "serve" },
  },
}
EOF
