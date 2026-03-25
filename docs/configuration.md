# Configuration

## File locations and precedence

Configuration is loaded from multiple sources, with later sources overriding earlier ones:

1. **Compiled defaults** — built into the binary
2. **User config** — `~/.config/weavr/config.toml` (XDG base directory)
3. **Project config** — `.weavr.toml` in the current working directory
4. **`--config PATH`** — explicit config file (replaces project config)
5. **CLI flags** — highest priority

Most sections use **field-level merging**: a higher-priority layer only overrides the specific fields it sets. Exceptions are noted below.

## Full annotated example

```toml
[theme]
name = "dark"                # Built-in theme name (see docs/themes.md)
# custom = { ... }           # OR inline custom theme (mutually exclusive with name)

[strategies]
default = "left"             # Default resolution: left, right, both, ast
deduplicate = false          # Deduplicate when using accept-both

[headless]
fail_on_ambiguous = false    # Exit 1 if any hunk can't be auto-resolved

[git]
auto_stage = false           # Automatically stage resolved files
stage_prompt = true          # Prompt to stage after resolution

[keybindings]
next_hunk = ["j", "<Down>"]  # Single key or array of keys
prev_hunk = ["k", "<Up>"]
resolve_left = "o"
resolve_right = "t"

# Requires jj feature
[jj]
squash_after_resolve = false # Run jj squash after all hunks resolved

# Requires ai feature
[ai]
enabled = true
provider = "claude"          # claude, openai, or local
timeout = 30
min_confidence = 0.7
auto_suggest = false

# Requires ast feature
[ast]
# AST-specific configuration (passed to weavr-ast)
```

## Sections

### `[theme]`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | string | `"dark"` | Built-in theme name ([full list](themes.md)) |
| `custom` | table | — | Inline custom theme definition |

`name` and `custom` are **mutually exclusive**. If both are set, the parser rejects the config.

**Merge behavior:** Full override. If a higher-priority layer sets `name`, any `custom` from a lower layer is discarded (and vice versa). If the higher layer has an empty `[theme]` section, the lower layer's theme is used.

### `[strategies]`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `default` | string | `"left"` | Default resolution strategy: `left`, `right`, `both`, `ast` |
| `deduplicate` | bool | `false` | Deduplicate lines when using accept-both |

### `[headless]`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `fail_on_ambiguous` | bool | `false` | Exit with code 1 if any hunk has no auto-resolution |

### `[git]`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `auto_stage` | bool | `false` | Automatically `git add` resolved files |
| `stage_prompt` | bool | `true` | Prompt to stage after resolution completes |

### `[keybindings]`

Action names use `snake_case`. Values are either a single key notation string or an array of strings.

```toml
[keybindings]
next_hunk = "j"              # Single key
next_hunk = ["j", "<Down>"]  # Multiple keys for the same action
```

**Key notation format:**
- Single characters: `"j"`, `"k"`, `"o"`
- Special keys: `"<Down>"`, `"<Up>"`, `"<Tab>"`, `"<Enter>"`, `"<Esc>"`, `"<PageDown>"`, `"<PageUp>"`, `"<F1>"`
- Modifiers: `"<C-d>"` (Ctrl+d), `"<S-Tab>"` (Shift+Tab)

**Merge behavior:** Action-level overlay. A higher-priority layer replaces all bindings for an action it specifies; lower-layer bindings for other actions are preserved.

See [TUI usage](tui.md) for the full list of default bindings and action names.

### `[jj]`

*Requires the `jj` feature.*

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `squash_after_resolve` | bool | `false` | Run `jj squash` after all hunks in a file are resolved |

### `[ai]`

*Requires the `ai` feature.* See [AI integration](ai.md) for full details.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | bool | — | Enable AI suggestions |
| `provider` | string | — | Provider: `claude`, `openai`, or `local` |
| `timeout` | int | — | Request timeout in seconds |
| `min_confidence` | float | — | Minimum confidence threshold |
| `auto_suggest` | bool | — | Automatically suggest on hunk focus |

### `[ast]`

*Requires the `ast` feature.* See [AST merge](ast-merge.md) for details.

## Unknown sections

Unknown top-level sections are silently ignored. This allows feature-gated sections (like `[ai]`) to remain in config files even when the feature is not compiled in. Unknown fields within known sections are rejected.

## Creating a config file

`weavr init` creates a `.weavr.toml` with all options commented out:

```bash
weavr init
```

Or create one manually in your project root or at `~/.config/weavr/config.toml` for user-level defaults.
