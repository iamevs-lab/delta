# Thin wrapper: cargo build copies the CLI + plugin to %USERPROFILE%\.evs-delta\
$ErrorActionPreference = "Stop"

$Repo = Split-Path -Parent $PSScriptRoot
$HomeDelta = Join-Path $env:USERPROFILE ".evs-delta"

Write-Host "Building release delta-cli (installs to $HomeDelta)..."
Push-Location $Repo
try {
    cargo build --release -p delta-cli
    # First run copies the just-linked exe if the post-link waiter has not finished.
    cargo run --release -p delta-cli -- --version | Out-Null
} finally {
    Pop-Location
}

Write-Host ""
Write-Host "Installed:"
Write-Host "  CLI    $HomeDelta\bin\delta-cli.exe"
Write-Host "  plugin $HomeDelta\delta.nvim"
Write-Host ""
Write-Host "lazy.nvim:"
Write-Host @'
{
  name = "delta.nvim",
  dir = vim.fn.expand("~/.evs-delta/delta.nvim"),
  lazy = false,
  opts = {
    cmd = { vim.fn.expand("~/.evs-delta/bin/delta-cli.exe"), "serve" },
  },
}
'@
