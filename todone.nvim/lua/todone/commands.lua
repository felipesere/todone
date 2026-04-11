local M = {}

local function get_today_path()
  local result = vim.fn.system("todone today")
  if vim.v.shell_error ~= 0 then
    vim.notify("todone: " .. result, vim.log.levels.ERROR)
    return nil
  end
  return result:gsub("\n$", "")
end

function M.open_today()
  local path = get_today_path()
  if not path then return end
  vim.cmd.edit(path)
end

function M.add_todo()
  vim.ui.input({ prompt = "Todo: " }, function(text)
    if not text or text == "" then return end
    vim.ui.input({ prompt = "Project (default: inbox): " }, function(project)
      if project == nil then return end  -- user pressed <Esc>
      if project == "" then project = "inbox" end

      local cmd = string.format(
        "todone add %s --project %s",
        vim.fn.shellescape(text),
        vim.fn.shellescape(project)
      )
      local result = vim.fn.system(cmd)
      if vim.v.shell_error ~= 0 then
        vim.notify("todone: " .. result, vim.log.levels.ERROR)
        return
      end

      if require("todone")._opts.open_after_add then
        M.open_today()
      else
        vim.notify(string.format('Added "%s" → %s', text, project), vim.log.levels.INFO)
      end
    end)
  end)
end

function M.pick_project()
  local path = get_today_path()
  if not path then return end

  local ok, lines = pcall(vim.fn.readfile, path)
  if not ok then
    vim.notify("todone: could not read " .. path, vim.log.levels.ERROR)
    return
  end

  local sections = {}
  for i, line in ipairs(lines) do
    local name = line:match("^## (.+)$")
    if name then
      table.insert(sections, { name = name, lnum = i })
    end
  end

  if #sections == 0 then
    vim.notify("todone: no sections in today's file", vim.log.levels.WARN)
    return
  end

  vim.ui.select(sections, {
    prompt = "Jump to project:",
    format_item = function(s) return s.name end,
  }, function(choice)
    if not choice then return end
    vim.cmd.edit(path)
    vim.api.nvim_win_set_cursor(0, { choice.lnum, 0 })
  end)
end

return M
