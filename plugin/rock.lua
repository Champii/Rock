if vim.g.loaded_rock_nvim then
  return
end
vim.g.loaded_rock_nvim = true

vim.filetype.add({
  extension = {
    rk = "rock",
  },
})

vim.api.nvim_create_user_command("RockLspInfo", function()
  require("rock").info()
end, { desc = "Show Rock LSP configuration and clients" })

vim.api.nvim_create_user_command("RockLspRestart", function()
  require("rock").restart()
end, { desc = "Restart Rock LSP clients" })
