-- End-to-end probe: RPC + real buffer attach/edit.
local out = os.getenv("DELTA_PROBE_OUT") or "delta-probe.json"
local cli = os.getenv("DELTA_CLI") or "delta-cli"
local report = { cli = cli, errors = {} }

local function push_err(msg)
  report.errors[#report.errors + 1] = msg
end

vim.opt.runtimepath:prepend(vim.fn.fnamemodify(debug.getinfo(1, "S").source:sub(2), ":p:h:h"))

local ok_req, delta = pcall(require, "delta")
if not ok_req then
  push_err("require delta failed: " .. tostring(delta))
  vim.fn.writefile({ vim.json.encode(report) }, out)
  vim.cmd("qa!")
  return
end

delta.setup({ cmd = { cli, "serve" } })
local rpc = require("delta.rpc")
report.job_started = vim.wait(3000, function()
  return rpc.running()
end, 50) and rpc.running()

if not report.job_started then
  push_err("delta-cli job did not start")
  vim.fn.writefile({ vim.json.encode(report) }, out)
  vim.cmd("qa!")
  return
end

local function wait_rpc(cmd, payload, timeout)
  local got = nil
  rpc.request(cmd, payload, function(msg)
    got = msg
  end)
  vim.wait(timeout or 4000, function()
    return got ~= nil
  end, 20)
  return got
end

report.open = wait_rpc("open", { buf = 1, text = "let foo = 10;\nlet bar = 20;\n", path = "probe.evs" })
report.edit = wait_rpc("edit", {
  buf = 1,
  delta = { kind = "Insert", offset = 7, text = "x" },
})
report.status = wait_rpc("status", { buf = 1 })

local src = vim.api.nvim_create_buf(true, false)
vim.api.nvim_buf_set_name(src, "probe-live.evs")
vim.api.nvim_buf_set_lines(src, 0, -1, false, { "let foo = 10;", "let bar = 20;" })
vim.api.nvim_set_current_buf(src)
report.attached = vim.wait(2000, function()
  return vim.b[src].delta_attached == true
end, 20)

delta.last = {}
vim.api.nvim_buf_set_text(src, 0, 7, 0, 7, { "x" })
report.live_edit = nil
vim.wait(2000, function()
  local last = require("delta").last
  if last and last.ok and (tonumber(last.inserted_chars) or 0) > 0 then
    report.live_edit = last
    return true
  end
  report.live_edit = last
  return false
end, 20)

report.live_status = wait_rpc("status", { buf = src })

vim.fn.writefile({ vim.json.encode(report) }, out)
vim.cmd("qa!")
