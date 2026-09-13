local M = {}

local job_id = nil
local pending = {}
local next_id = 1
local buf = ""
local last_cmd = nil
local last_cwd = nil
local stderr_chunks = {}

local function decode_line(line)
  line = (line or ""):gsub("\r$", "")
  if line == "" then
    return nil
  end
  local ok, obj = pcall(vim.json.decode, line)
  if not ok then
    return nil
  end
  return obj
end

function M.running()
  return type(job_id) == "number" and job_id > 0
end

function M.ensure(cmd, cwd)
  if cmd then
    last_cmd = cmd
  end
  if cwd then
    last_cwd = cwd
  end
  if M.running() then
    return true
  end
  if not last_cmd then
    return false
  end

  job_id = nil
  pending = {}
  buf = ""
  stderr_chunks = {}

  job_id = vim.fn.jobstart(last_cmd, {
    rpc = false,
    stdin = "pipe",
    cwd = last_cwd,
    on_stdout = function(_, data)
      for _, chunk in ipairs(data) do
        if chunk == "" then
          if buf ~= "" then
            local msg = decode_line(buf)
            buf = ""
            if msg and msg.id and pending[msg.id] then
              local cb = pending[msg.id]
              pending[msg.id] = nil
              cb(msg)
            end
          end
        else
          buf = buf .. chunk
        end
      end
    end,
    on_stderr = function(_, data)
      for _, chunk in ipairs(data) do
        if chunk ~= "" then
          stderr_chunks[#stderr_chunks + 1] = chunk
        end
      end
    end,
    on_exit = function(id, code)
      if job_id ~= id then
        return
      end
      job_id = nil
      pending = {}
      buf = ""
      if code ~= 0 then
        local err = table.concat(stderr_chunks, "\n")
        vim.schedule(function()
          vim.notify(
            ("delta-cli exited %s%s"):format(code, err ~= "" and ("\n" .. err) or ""),
            vim.log.levels.ERROR
          )
        end)
      end
    end,
  })

  if not M.running() then
    job_id = nil
    vim.notify("failed to start delta-cli: " .. vim.inspect(last_cmd), vim.log.levels.ERROR)
    return false
  end
  return true
end

function M.stop()
  if M.running() then
    local id = job_id
    job_id = nil
    vim.fn.jobstop(id)
  end
end

function M.request(cmd, payload, cb)
  if not M.ensure(last_cmd, last_cwd) then
    if cb then
      cb({ ok = false, error = "delta-cli not running" })
    end
    return
  end
  local id = next_id
  next_id = next_id + 1
  payload = payload or {}
  payload.id = id
  payload.cmd = cmd
  if cb then
    pending[id] = cb
  end
  vim.fn.chansend(job_id, vim.json.encode(payload) .. "\n")
end

return M
