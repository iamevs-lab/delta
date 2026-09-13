if vim.g.loaded_evs_delta then
  return
end
vim.g.loaded_evs_delta = true

vim.api.nvim_create_autocmd("VimEnter", {
  once = true,
  callback = function()
    if not vim.g.delta_setup_done then
      require("delta").setup()
    end
  end,
})
