# todone

A no-fuss daily todo tracker. Plain markdown files, automatic carryover, and a small TUI for marking things done.

## Install

```sh
brew tap felipesere/tap
brew install todone
```

## How it works

Each day gets a markdown file at `~/.todone/notes/YYYY-MM-DD.md`. Todos live under project sections. When you start a new day, open and in-progress items carry over automatically — done items don't.

```markdown
# 2026-04-27

## inbox
- [ ] write the README
- [/] review PR from Alice
- [x] fix that weird bug

## work
- [ ] deploy to staging
```

## Commands

### `todone add [TEXT]`

Add a todo. Omit the text to be prompted.

```sh
todone add "fix the flaky test"
todone add -p work "deploy to staging"
```

Set `TODONE_PROJECT` to skip the project prompt.

### `todone list`

Show open todos for today.

```sh
todone list             # all projects
todone list -p work     # one project only
todone list -a          # all, ignoring TODONE_PROJECT
```

### `todone done`

Interactive TUI to cycle todo states. Navigate with `j`/`k`, toggle with `Space` or `Enter`, quit with `q`.

States cycle: `[ ]` open → `[/]` in progress → `[x]` done → back to open.

### `todone carry`

Explicitly carry over todos from the previous day. Safe to run multiple times.

### `todone today`

Print the path to today's file — handy for opening it in your editor.

```sh
nvim $(todone today)
```

## Inline highlights

Todos support inline annotations that colour output:

- `@mentions` — magenta
- `#tags` — cyan
- `` `code` `` — bold

## Configuration

| Variable        | Default              | Description                          |
|-----------------|----------------------|--------------------------------------|
| `TODONE_DIR`    | `~/.todone/notes`    | Where markdown files are stored      |
| `TODONE_PROJECT`| `inbox`              | Default project for `add` and `list` |
