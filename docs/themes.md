# Themes & Customization

## Built-in themes

weavr ships with 19 built-in themes:

| Theme | Name |
|-------|------|
| Dark *(default)* | `dark` |
| Light | `light` |
| Catppuccin Latte | `catppuccin-latte` |
| Catppuccin Frappe | `catppuccin-frappe` |
| Catppuccin Macchiato | `catppuccin-macchiato` |
| Catppuccin Mocha | `catppuccin-mocha` |
| Dracula | `dracula` |
| Gruvbox Dark | `gruvbox-dark` |
| Gruvbox Light | `gruvbox-light` |
| Nord | `nord` |
| One Dark | `one-dark` |
| Rose Pine | `rose-pine` |
| Rose Pine Moon | `rose-pine-moon` |
| Rose Pine Dawn | `rose-pine-dawn` |
| Solarized Dark | `solarized-dark` |
| Solarized Light | `solarized-light` |
| Tokyo Night | `tokyo-night` |
| Tokyo Night Storm | `tokyo-night-storm` |
| Tokyo Night Light | `tokyo-night-light` |

## Selecting a theme

**CLI flag:**

```bash
weavr --theme dracula
```

**Config file:**

```toml
[theme]
name = "dracula"
```

The CLI flag overrides the config file.

## Custom themes

Define a custom theme inline in your config file using the `custom` field instead of `name`:

```toml
[theme.custom]

[theme.custom.base]
background = "#1a1b26"
foreground = "#c0caf5"
muted = "#565f89"
accent = "#7aa2f7"
secondary = "#9ece6a"

[theme.custom.conflict]
left = { fg = "#7aa2f7" }
right = { fg = "#f7768e" }
both = { fg = "#9ece6a" }
base = { fg = "#bb9af7" }
unresolved = { fg = "#f7768e" }
resolved = { fg = "#9ece6a" }

[theme.custom.diff]
added = { fg = "#9ece6a" }
removed = { fg = "#f7768e" }
modified = { fg = "#e0af68" }
context = { fg = "#565f89" }

[theme.custom.ui]
border_focused = "#7aa2f7"
border_unfocused = "#565f89"
title = { fg = "#c0caf5" }
status = { fg = "#565f89" }
selection = { fg = "#c0caf5", bg = "#283457" }
```

`name` and `custom` are **mutually exclusive** — set one or the other, not both.

## Theme format reference

A custom theme has four required sections:

### `[base]` — Color palette

| Field | Type | Description |
|-------|------|-------------|
| `background` | `"#RRGGBB"` | Main background color |
| `foreground` | `"#RRGGBB"` | Main text color |
| `muted` | `"#RRGGBB"` | Subdued text (comments, hints) |
| `accent` | `"#RRGGBB"` | Primary accent color |
| `secondary` | `"#RRGGBB"` | Secondary accent color |

### `[conflict]` — Conflict visualization

| Field | Type | Description |
|-------|------|-------------|
| `left` | style | Ours/left side |
| `right` | style | Theirs/right side |
| `both` | style | Accept-both |
| `base` | style | Base/ancestor |
| `unresolved` | style | Unresolved hunk indicator |
| `resolved` | style | Resolved hunk indicator |

### `[diff]` — Diff visualization

| Field | Type | Description |
|-------|------|-------------|
| `added` | style | Added lines |
| `removed` | style | Removed lines |
| `modified` | style | Modified lines |
| `context` | style | Unchanged context lines |

### `[ui]` — UI elements

| Field | Type | Description |
|-------|------|-------------|
| `border_focused` | `"#RRGGBB"` | Focused pane border color |
| `border_unfocused` | `"#RRGGBB"` | Unfocused pane border color |
| `title` | style | Title bar |
| `status` | style | Status bar |
| `selection` | style | Selected item highlight |

### Style format

Style fields are objects with optional `fg` and `bg`:

```toml
# Foreground only
left = { fg = "#7aa2f7" }

# Foreground and background
selection = { fg = "#c0caf5", bg = "#283457" }
```

Colors must be `"#RRGGBB"` hex strings.

## Config precedence

Theme selection follows the standard [configuration precedence](configuration.md). When a higher-priority layer specifies a theme, it **fully overrides** the lower layer's theme (no per-field merging).
