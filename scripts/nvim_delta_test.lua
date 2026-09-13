-- Headless check: open a buffer, insert one character, confirm the engine saw it.
-- nvim --headless --clean -c "set rtp^=neovim/delta.nvim" -c "luafile scripts/nvim_delta_test.lua"

local root = vim.fn.fnamemodify(vim.fn.getcwd() .. "/neovim/delta.nvim", ":p")
vim.opt.runtimepath:prepend(root)
package.path = root .. "lua/?.lua;" .. root .. "lua/?/init.lua;" .. package.path

local delta = require("delta")
local exe = vim.fn.fnamemodify("target/debug/delta-cli.exe", ":p")
if vim.fn.filereadable(exe) == 0 then
  exe = vim.fn.fnamemodify("target/debug/delta-cli", ":p")
end
delta.setup({ cmd = { exe, "serve" } })

local outfile = vim.fn.fnamemodify("benchmarks/results/nvim-headless.json", ":p")
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

local rpc = require("delta.rpc")
if not vim.wait(4000, function()
  return rpc.running()
end, 50) then
  finish({ ok = false, error = "delta-cli did not start", cmd = exe })
  return
end

rpc.request("open", {
  buf = buf,
  text = table.concat(vim.api.nvim_buf_get_lines(buf, 0, -1, false), "\n"),
}, function(open_msg)
  if not open_msg or open_msg.ok == false then
    finish({ ok = false, error = "open failed: " .. tostring(open_msg and open_msg.error) })
    return
  end

  rpc.request("edit", {
    buf = buf,
    delta = { kind = "Insert", offset = 4, text = "x" },
  }, function(edit_msg)
    if not edit_msg or edit_msg.ok == false then
      finish({
        ok = false,
        error = "edit failed: " .. tostring(edit_msg and edit_msg.error),
        edit = edit_msg,
      })
      return
    end
    rpc.request("inspect", { buf = buf }, function(inspect)
      rpc.request("status", { buf = buf }, function(status)
        local expected = #"let xfoo = 10;\nlet bar = 20;"
        local source_ok = inspect and inspect.source_len == expected
        local change_ok = edit_msg.inserted_chars == 1
        finish({
          ok = source_ok and change_ok,
          detail = string.format(
            "inserted=%s source_len=%s expected=%s reused=%.1f time_ms=%.3f",
            tostring(edit_msg.inserted_chars),
            tostring(inspect and inspect.source_len),
            tostring(expected),
            tonumber(edit_msg.reused_percent) or 0,
            tonumber(edit_msg.time_ms) or 0
          ),
          edit = edit_msg,
          inspect = inspect,
          status = status,
          source_ok = source_ok,
          change_ok = change_ok,
        })
      end)
    end)
  end)
end)
