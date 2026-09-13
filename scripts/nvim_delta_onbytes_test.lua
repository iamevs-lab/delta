-- Verify on_bytes argument order: a buffer edit must become a correct Delta.
local root = vim.fn.fnamemodify(vim.fn.getcwd() .. "/neovim/delta.nvim", ":p")
vim.opt.runtimepath:prepend(root)
package.path = root .. "lua/?.lua;" .. root .. "lua/?/init.lua;" .. package.path

local delta = require("delta")
local exe = vim.fn.fnamemodify("target/debug/delta-cli.exe", ":p")
if vim.fn.filereadable(exe) == 0 then
  exe = vim.fn.fnamemodify("target/debug/delta-cli", ":p")
end

local outfile = vim.fn.fnamemodify("benchmarks/results/nvim-onbytes.json", ":p")
vim.fn.mkdir(vim.fn.fnamemodify(outfile, ":h"), "p")

local function finish(obj)
  vim.fn.writefile({ vim.json.encode(obj) }, outfile)
  if obj.ok then
    print("DELTA_OK " .. (obj.detail or ""))
    vim.cmd("qa!")
  else
    print("DELTA_FAIL " .. (obj.error or vim.inspect(obj)))
    vim.cmd("cquit 1")
  end
end

vim.cmd("enew")
local buf = vim.api.nvim_get_current_buf()
vim.api.nvim_buf_set_lines(buf, 0, -1, false, {
  "let foo = 10;",
  "let bar = 20;",
})

delta.setup({ cmd = { exe, "serve" } })

local rpc = require("delta.rpc")
if not vim.wait(4000, function()
  return rpc.running()
end, 50) then
  finish({ ok = false, error = "delta-cli did not start" })
  return
end

-- Wait until the server has accepted this buffer, not just local attach.
if not vim.wait(3000, function()
  local last = require("delta").last
  return last and last.opened
end, 50) then
  finish({ ok = false, error = "open never completed", last = require("delta").last })
  return
end

-- Real editor-shaped change. on_bytes must map this to Insert{offset=4, text="x"}.
vim.api.nvim_buf_set_text(buf, 0, 4, 0, 4, { "x" })

if not vim.wait(2000, function()
  local last = require("delta").last
  return last and last.inserted_chars == 1
end, 50) then
  finish({
    ok = false,
    error = "on_bytes did not produce inserted_chars=1",
    last = require("delta").last,
  })
  return
end

rpc.request("inspect", { buf = buf }, function(inspect)
  local expected = #"let xfoo = 10;\nlet bar = 20;"
  local source_ok = inspect and inspect.source_len == expected
  finish({
    ok = source_ok,
    detail = string.format(
      "source_len=%s expected=%s last_insert=%s",
      tostring(inspect and inspect.source_len),
      tostring(expected),
      tostring(require("delta").last.inserted_chars)
    ),
    inspect = inspect,
    last = require("delta").last,
  })
end)
