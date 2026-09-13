-- Print the real on_bytes argument list for one insert.
vim.cmd("enew")
local buf = vim.api.nvim_get_current_buf()
vim.api.nvim_buf_set_lines(buf, 0, -1, false, { "let foo = 10;" })
local dumped = nil
vim.api.nvim_buf_attach(buf, false, {
  on_bytes = function(...)
    dumped = { ... }
    return false
  end,
})
vim.api.nvim_buf_set_text(buf, 0, 4, 0, 4, { "x" })
vim.wait(100)
local out = vim.fn.fnamemodify("benchmarks/results/onbytes-args.json", ":p")
vim.fn.writefile({ vim.json.encode(dumped) }, out)
print("ARGS " .. vim.inspect(dumped))
vim.cmd("qa!")
