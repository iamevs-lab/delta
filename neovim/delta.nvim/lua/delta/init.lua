local rpc = require("delta.rpc")
local ui = require("delta.ui")
local discover = require("delta.discover")

local M = {
  enabled = true,
  last = {},
  opts = {},
}

local function buf_text(buf)
  local lines = vim.api.nvim_buf_get_lines(buf, 0, -1, false)
  return table.concat(lines, "\n")
end

local function remember(msg)
  if not msg then
    return
  end
  M.last = msg
  vim.g.delta_statusline = ui.statusline(msg)
end

-- nvim_buf_attach on_bytes:
--   buf, tick, start_row, start_col, start_byte,
--   old_end_row, old_end_col, old_end_byte,
--   new_end_row, new_end_col, new_end_byte
local function on_bytes(
  buf,
  _tick,
  start_row,
  start_col,
  start_byte,
  _old_end_row,
  _old_end_col,
  old_end_byte,
  new_end_row,
  new_end_col,
  new_end_byte
)
  if not M.enabled then
    return false
  end

  local inserted = ""
  if new_end_byte > 0 then
    local end_row = start_row + new_end_row
    local end_col = new_end_col
    if new_end_row == 0 then
      end_col = start_col + new_end_col
    end
    local ok, parts = pcall(vim.api.nvim_buf_get_text, buf, start_row, start_col, end_row, end_col, {})
    if ok then
      inserted = table.concat(parts, "\n")
    end
  end

  local delta
  if old_end_byte == 0 and new_end_byte > 0 then
    delta = { kind = "Insert", offset = start_byte, text = inserted }
  elseif new_end_byte == 0 and old_end_byte > 0 then
    delta = { kind = "Delete", start = start_byte, ["end"] = start_byte + old_end_byte }
  else
    delta = {
      kind = "Replace",
      start = start_byte,
      ["end"] = start_byte + old_end_byte,
      text = inserted,
    }
  end

  -- on_bytes is a fast callback; sending on the job channel must be deferred.
  vim.schedule(function()
    rpc.request("edit", { buf = buf, delta = delta }, function(msg)
      remember(msg)
      if msg and msg.ok == false then
        vim.notify("delta edit: " .. (msg.error or "failed"), vim.log.levels.WARN)
      end
    end)
  end)
  return false
end

local function should_attach(buf)
  if not vim.api.nvim_buf_is_valid(buf) then
    return false
  end
  if vim.b[buf].delta_ui then
    return false
  end
  if vim.bo[buf].buftype ~= "" then
    return false
  end
  local name = vim.api.nvim_buf_get_name(buf)
  local lines = vim.api.nvim_buf_get_lines(buf, 0, -1, false)
  local empty = #lines == 0 or (#lines == 1 and lines[1] == "")
  -- Do not attach the startup [No Name] buffer. It was wiping real sessions
  -- by reopening buffer 1 as an empty pipeline.
  if empty and (name == "" or vim.fn.filereadable(name) == 0) then
    return false
  end
  return true
end

local function attach(buf)
  if not should_attach(buf) then
    return
  end
  if vim.b[buf].delta_attached then
    return
  end
  vim.b[buf].delta_attached = true
  rpc.request("open", { buf = buf, path = vim.api.nvim_buf_get_name(buf), text = buf_text(buf) }, function(msg)
    remember(msg)
    if msg and msg.ok == false then
      vim.notify("delta open: " .. (msg.error or "failed"), vim.log.levels.ERROR)
    end
  end)
  vim.api.nvim_buf_attach(buf, false, {
    -- Neovim 0.12 prefixes the event name ("bytes"); older builds start at buf.
    on_bytes = function(event_or_buf, ...)
      if event_or_buf == "bytes" then
        return on_bytes(...)
      end
      return on_bytes(event_or_buf, ...)
    end,
  })
end

local function reopen(buf)
  if not should_attach(buf) then
    return
  end
  if vim.b[buf].delta_attached then
    rpc.request("open", { buf = buf, path = vim.api.nvim_buf_get_name(buf), text = buf_text(buf) }, remember)
    return
  end
  attach(buf)
end

function M.setup(opts)
  M.enabled = true
  M.opts = vim.tbl_deep_extend("force", M.opts, opts or {})
  vim.g.delta_setup_done = true

  local cmd, cwd = discover.command(M.opts.cmd)
  M.opts.resolved_cmd = cmd
  rpc.ensure(cmd, cwd)

  local group = vim.api.nvim_create_augroup("EvsDelta", { clear = true })

  vim.api.nvim_create_autocmd({ "BufEnter", "BufNewFile" }, {
    group = group,
    callback = function(ev)
      if M.enabled then
        attach(ev.buf)
      end
    end,
  })

  vim.api.nvim_create_autocmd("BufRead", {
    group = group,
    callback = function(ev)
      if M.enabled then
        reopen(ev.buf)
      end
    end,
  })

  vim.api.nvim_create_autocmd({ "TextChanged", "TextChangedI" }, {
    group = group,
    callback = function(ev)
      if M.enabled and not vim.b[ev.buf].delta_attached then
        attach(ev.buf)
      end
    end,
  })

  vim.api.nvim_create_autocmd("BufDelete", {
    group = group,
    callback = function(ev)
      if vim.b[ev.buf].delta_ui then
        return
      end
      if vim.b[ev.buf].delta_attached then
        rpc.request("close", { buf = ev.buf })
      end
    end,
  })

  vim.api.nvim_create_user_command("DeltaStatus", function()
    rpc.request("status", { buf = vim.api.nvim_get_current_buf() }, function(msg)
      remember(msg)
      ui.float(ui.status_lines(msg), "DELTA")
    end)
  end, {})

  vim.api.nvim_create_user_command("DeltaInspect", function()
    rpc.request("inspect", { buf = vim.api.nvim_get_current_buf() }, function(msg)
      ui.float(vim.split(vim.inspect(msg), "\n"), "DELTA inspect")
    end)
  end, {})

  vim.api.nvim_create_user_command("DeltaGraph", function()
    rpc.request("graph", { buf = vim.api.nvim_get_current_buf() }, function(msg)
      local lines = {}
      if msg.ascii then
        for line in tostring(msg.ascii):gmatch("[^\n]+") do
          table.insert(lines, line)
        end
      else
        lines = vim.split(vim.inspect(msg), "\n")
      end
      ui.float(lines, "DELTA graph")
    end)
  end, {})

  vim.api.nvim_create_user_command("DeltaStats", function()
    rpc.request("stats", { buf = vim.api.nvim_get_current_buf() }, function(msg)
      ui.float(vim.split(vim.inspect(msg), "\n"), "DELTA stats")
    end)
  end, {})

  vim.api.nvim_create_user_command("DeltaBenchmark", function()
    ui.float({
      "Benchmarks are not invented in the UI.",
      "Run from the repo:",
      "",
      "  cargo run -p delta-cli -- benchmark --workload text-smoke --out benchmarks/results",
    }, "DELTA bench")
  end, {})

  vim.api.nvim_create_user_command("DeltaReset", function()
    local buf = vim.api.nvim_get_current_buf()
    rpc.request("reset", { buf = buf }, function()
      reopen(buf)
    end)
    M.last = {}
  end, {})

  vim.api.nvim_create_user_command("DeltaToggle", function()
    M.enabled = not M.enabled
    if M.enabled then
      local resolved, resolved_cwd = discover.command(M.opts.cmd)
      rpc.ensure(resolved, resolved_cwd)
      reopen(vim.api.nvim_get_current_buf())
    end
    vim.notify("delta " .. (M.enabled and "on" or "off"))
  end, {})

  vim.schedule(function()
    if M.enabled then
      attach(vim.api.nvim_get_current_buf())
    end
  end)
end

return M
