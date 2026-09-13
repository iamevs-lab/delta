local M = {}

local function plugin_root()
  local src = debug.getinfo(1, "S").source:sub(2)
  return vim.fn.fnamemodify(src, ":p:h:h:h")
end

function M.workspace_root()
  return vim.fn.fnamemodify(plugin_root(), ":h:h")
end

local function exe_names()
  if vim.fn.has("win32") == 1 then
    return { "delta-cli.exe", "delta-cli" }
  end
  return { "delta-cli" }
end

local function readable(path)
  return path and vim.fn.filereadable(path) == 1
end

function M.home_dir()
  return vim.fn.expand("~/.evs-delta")
end

function M.home_cli()
  local bin = M.home_dir() .. "/bin/"
  if vim.fn.has("win32") == 1 then
    local exe = bin .. "delta-cli.exe"
    if readable(exe) then
      return exe
    end
  end
  local unix = bin .. "delta-cli"
  if readable(unix) then
    return unix
  end
  return nil
end

function M.command(explicit)
  if type(explicit) == "table" and #explicit > 0 then
    return explicit, M.workspace_root()
  end
  if type(explicit) == "string" and explicit ~= "" then
    return { explicit, "serve" }, M.workspace_root()
  end

  local home_cli = M.home_cli()
  if home_cli then
    return { home_cli, "serve" }, M.workspace_root()
  end

  local names = exe_names()
  local roots = { M.workspace_root() }
  local cargo_target = os.getenv("CARGO_TARGET_DIR")
  if cargo_target and cargo_target ~= "" then
    table.insert(roots, cargo_target)
  end

  local candidates = {}
  for _, root in ipairs(roots) do
    for _, name in ipairs(names) do
      table.insert(candidates, root .. "/target/release/" .. name)
      table.insert(candidates, root .. "/target/debug/" .. name)
      table.insert(candidates, root .. "/target-nvim/release/" .. name)
      table.insert(candidates, root .. "/target-nvim/debug/" .. name)
      -- CARGO_TARGET_DIR is already a target dir
      table.insert(candidates, root .. "/release/" .. name)
      table.insert(candidates, root .. "/debug/" .. name)
    end
  end

  for _, path in ipairs(candidates) do
    if readable(path) then
      return { path, "serve" }, M.workspace_root()
    end
  end

  if vim.fn.executable("delta-cli") == 1 then
    return { "delta-cli", "serve" }, M.workspace_root()
  end

  return { "cargo", "run", "--quiet", "-p", "delta-cli", "--", "serve" }, M.workspace_root()
end

return M
