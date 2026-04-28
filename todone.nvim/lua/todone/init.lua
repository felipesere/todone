local M = {}

M._opts = {}

local defaults = {
	keys = {
		open_today = "<leader>tt",
		add_todo = "<leader>ta",
		pick_project = "<leader>tp",
	},
	-- After adding a todo, open today's file in the current window.
	open_after_add = true,
}

function M.setup(opts)
	M._opts = vim.tbl_deep_extend("force", defaults, opts or {})
	local cmds = require("todone.commands")
	vim.keymap.set("n", M._opts.keys.open_today, cmds.open_today, { desc = "todone: open today's file" })
	vim.keymap.set("n", M._opts.keys.add_todo, cmds.add_todo, { desc = "todone: add todo" })
	vim.keymap.set("n", M._opts.keys.pick_project, cmds.pick_project, { desc = "todone: jump to project" })
end

return M
