-- Drive the user's real Neovim config against examples/dummy-lab/main.let
-- and write engine-produced stats to examples/dummy-lab/report.json

local report_path = vim.fn.fnamemodify("examples/dummy-lab/report.json", ":p")
vim.fn.mkdir(vim.fn.fnamemodify(report_path, ":h"), "p")

local function write_and_quit(obj, ok)
  vim.fn.writefile({ vim.json.encode(obj) }, report_path)
  if ok then
    print("DELTA_LIVE_OK " .. (obj.summary or ""))
    vim.cmd("qa!")
  else
    print("DELTA_LIVE_FAIL " .. (obj.error or vim.inspect(obj)))
    vim.cmd("cquit 1")
  end
end

local loaded = vim.wait(20000, function()
  local ok = pcall(require, "delta")
  if not ok then
    return false
  end
  local rpc_ok, rpc = pcall(require, "delta.rpc")
  return rpc_ok and rpc.running()
end, 100)

if not loaded then
  local delta_ok, delta = pcall(require, "delta")
  write_and_quit({
    ok = false,
    error = "delta did not become ready under the user Neovim config",
    delta_module = delta_ok,
    resolved_cmd = delta_ok and delta.opts and delta.opts.resolved_cmd or nil,
    setup_done = vim.g.delta_setup_done,
  }, false)
  return
end

local delta = require("delta")
local rpc = require("delta.rpc")
local buf = vim.api.nvim_get_current_buf()

if not vim.wait(5000, function()
  return vim.b[buf].delta_attached == true
end, 50) then
  -- Force the same path a user gets after opening a file.
  vim.cmd("doautocmd BufRead")
  vim.cmd("doautocmd BufEnter")
end

if not vim.wait(5000, function()
  return vim.b[buf].delta_attached == true
end, 50) then
  write_and_quit({
    ok = false,
    error = "buffer never attached",
    buf = buf,
    name = vim.api.nvim_buf_get_name(buf),
    resolved_cmd = delta.opts.resolved_cmd,
  }, false)
  return
end

if not vim.wait(5000, function()
  return delta.last and (delta.last.opened or delta.last.source_len)
end, 50) then
  rpc.request("open", {
    buf = buf,
    path = vim.api.nvim_buf_get_name(buf),
    text = table.concat(vim.api.nvim_buf_get_lines(buf, 0, -1, false), "\n"),
  }, function(msg)
    delta.last = msg
  end)
  vim.wait(1500)
end

-- Local identifier edit: foo -> foox (one extra character at the first foo).
local lines = vim.api.nvim_buf_get_lines(buf, 0, -1, false)
local row, col
for i, line in ipairs(lines) do
  local s = line:find("foo", 1, true)
  if s then
    row, col = i - 1, s + 2
    break
  end
end
if not row then
  write_and_quit({ ok = false, error = "could not find foo in dummy program" }, false)
  return
end

vim.api.nvim_buf_set_text(buf, row, col, row, col, { "x" })

if not vim.wait(4000, function()
  return delta.last and delta.last.inserted_chars == 1
end, 50) then
  write_and_quit({
    ok = false,
    error = "edit did not produce inserted_chars=1",
    last = delta.last,
    resolved_cmd = delta.opts.resolved_cmd,
    attached = vim.b[buf].delta_attached,
  }, false)
  return
end

rpc.request("inspect", { buf = buf }, function(inspect)
  rpc.request("status", { buf = buf }, function(status)
    rpc.request("stats", { buf = buf }, function(stats)
      rpc.request("graph", { buf = buf }, function(graph)
        local after = table.concat(vim.api.nvim_buf_get_lines(buf, 0, -1, false), "\n")
        local engine_source_ok = inspect and type(inspect.source_len) == "number" and inspect.source_len == #after
        local ok = status
          and status.ok ~= false
          and status.inserted_chars == 1
          and engine_source_ok
        write_and_quit({
          ok = ok,
          summary = string.format(
            "insert=+1 reused=%.1f%% computed=%.1f%% tokens %s/%s ast %s/%s time=%.3fms source=%s",
            tonumber(status.reused_percent) or 0,
            tonumber(status.computed_percent) or 0,
            tostring(status.tokens_reused),
            tostring(status.tokens_recomputed),
            tostring(status.ast_reused),
            tostring(status.ast_recomputed),
            tonumber(status.time_ms) or 0,
            tostring(inspect and inspect.source_len)
          ),
          config = {
            setup_done = vim.g.delta_setup_done,
            resolved_cmd = delta.opts.resolved_cmd,
            attached = vim.b[buf].delta_attached,
            file = vim.api.nvim_buf_get_name(buf),
          },
          inspect = inspect,
          status = status,
          stats = stats,
          graph = graph and graph.ascii or nil,
          engine_source_matches_buffer = engine_source_ok,
        }, ok)
      end)
    end)
  end)
end)
