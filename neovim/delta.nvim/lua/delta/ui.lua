local M = {}

local function n(v)
  if v == nil then
    return "n/a"
  end
  return tostring(v)
end

local function pct(v)
  if type(v) ~= "number" then
    return "n/a"
  end
  return string.format("%.1f%%", v)
end

function M.status_lines(s)
  s = s or {}
  if s.ok == false then
    return {
      "DELTA",
      "",
      "Error",
      "  " .. (s.error or "unknown"),
    }
  end
  return {
    "DELTA",
    "",
    "Last change",
    string.format("  +%s -%s chars", n(s.inserted_chars), n(s.deleted_chars)),
    "",
    "Affected",
    string.format("  %s tokens", n(s.tokens_recomputed)),
    string.format("  %s AST nodes", n(s.ast_recomputed)),
    string.format("  %s symbols", n(s.symbols_recomputed)),
    string.format("  %s diagnostics", n(s.diagnostics_recomputed)),
    "",
    "Reused",
    string.format("  %s", pct(s.reused_percent)),
    "",
    "Computed",
    string.format("  %s", pct(s.computed_percent)),
    "",
    "Invalidated",
    string.format("  %s nodes", n(s.invalidated_nodes)),
    "",
    "Reused nodes",
    string.format("  %s", n(s.reused_nodes)),
    "",
    "Recomputed",
    string.format("  %s nodes", n(s.recomputed_nodes)),
    "",
    "Time",
    string.format("  %.3f ms", tonumber(s.time_ms) or 0),
  }
end

function M.float(lines, title)
  local buf = vim.api.nvim_create_buf(false, true)
  vim.b[buf].delta_ui = true
  vim.api.nvim_buf_set_lines(buf, 0, -1, false, lines)
  vim.bo[buf].buftype = "nofile"
  vim.bo[buf].bufhidden = "wipe"
  vim.bo[buf].modifiable = false
  local width = 40
  for _, line in ipairs(lines) do
    width = math.max(width, #line + 2)
  end
  local height = math.min(#lines + 2, 24)
  local opts = {
    relative = "editor",
    width = width,
    height = height,
    row = 2,
    col = math.max(0, vim.o.columns - width - 2),
    style = "minimal",
    border = "single",
    title = title or "DELTA",
    title_pos = "center",
  }
  vim.api.nvim_open_win(buf, true, opts)
end

function M.statusline(s)
  if not s or s.ok == false then
    return "DELTA err"
  end
  return string.format(
    "DELTA +%s -%s  reused %s  %s ms",
    n(s.inserted_chars),
    n(s.deleted_chars),
    pct(s.reused_percent),
    string.format("%.2f", tonumber(s.time_ms) or 0)
  )
end

return M
