# TUI Usage & Keybindings

## Layout

The TUI presents a three-pane merge view:

```
┌─ Ours (left) ──────┬─ Theirs (right) ─────┬─ Result ─────────────┐
│                     │                      │                      │
│  left/ours content  │  right/theirs content│  merged result       │
│                     │                      │                      │
├─ Base (optional) ───┴──────────────────────┴──────────────────────┤
│  common ancestor (toggle with Ctrl+b)                             │
├───────────────────────────────────────────────────────────────────┤
│  status bar: file name, hunk N/M, resolution state                │
└───────────────────────────────────────────────────────────────────┘
```

The status bar shows the current file, hunk position, and resolution progress.

## Keybindings

All keybindings shown below are defaults. They can be customized via the `[keybindings]` section in your [configuration](configuration.md).

### Resolution

| Key | Action |
|-----|--------|
| `o` | Accept ours (left) |
| `t` | Accept theirs (right) |
| `b` | Accept both (default order) |
| `B` | Accept both (options dialog) |
| `e` | Edit in `$EDITOR` |
| `i` | Edit in result pane (inline) |
| `x` | Clear resolution |
| `u` | Undo last action |
| `Ctrl+r` | Redo last action |

### Navigation

| Key | Action |
|-----|--------|
| `j` / `Down` | Next hunk |
| `k` / `Up` | Previous hunk |
| `n` | Next unresolved hunk |
| `N` | Previous unresolved hunk |
| `gg` | First hunk |
| `G` | Last hunk |
| `Tab` | Cycle panes forward |
| `Shift+Tab` | Cycle panes backward |
| `Enter` | Focus result pane |

### Scrolling

| Key | Action |
|-----|--------|
| `Ctrl+d` | Scroll half page down |
| `Ctrl+u` | Scroll half page up |
| `PageDown` | Scroll full page down |
| `PageUp` | Scroll full page up |

### Display

| Key | Action |
|-----|--------|
| `w` | Toggle word diff |
| `Ctrl+b` | Toggle base pane (diff3 view) |
| `h` | Toggle syntax highlighting |

### AI (when configured)

Requires the `ai` feature. See [AI integration](ai.md).

| Key | Action |
|-----|--------|
| `s` | AI suggest (current hunk) |
| `S` | AI suggest (all unresolved) |
| `?` | Help / AI explain (when suggestion shown) |
| `Enter` | Accept AI suggestion |
| `Esc` | Dismiss AI suggestion |

### AST merge

Requires the `ast` feature. See [AST merge](ast-merge.md).

| Key | Action |
|-----|--------|
| `a` | AST merge (current hunk) |
| `A` | AST merge (all unresolved) |
| `Enter` | Accept AST suggestion |

## Command mode

Press `:` to enter command mode (Vim-style).

| Command | Action |
|---------|--------|
| `:w` | Save file |
| `:q` | Quit (fails if unresolved hunks remain) |
| `:wq` | Save and quit |
| `:w!` | Save with unresolved hunks (conflict markers preserved) |
| `:wq!` | Save with unresolved hunks and quit |
| `:q!` | Force quit (discard changes) |
| `:help` | Show help overlay |

## Multi-file workflow

When weavr opens multiple conflicted files, use these commands to navigate between them:

| Command | Action |
|---------|--------|
| `:n` | Next file |
| `:prev` | Previous file |
| `:files` | Show file list |
| `:file N` | Jump to file N |

## Help overlay

Press `F1` to toggle the help overlay, which shows all keybindings reflecting your current configuration (including any customizations).

## Customizing keybindings

Keybindings are configured in the `[keybindings]` section of your config file. Action names use `snake_case` and accept either a single key string or an array:

```toml
[keybindings]
next_hunk = ["j", "<Down>"]
resolve_left = "a"
```

See [Configuration](configuration.md) for the full list of action names and key notation format.
