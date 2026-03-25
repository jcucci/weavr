# CLI Reference

## Usage

```
weavr [OPTIONS] [FILE...]
weavr <COMMAND>
```

## Modes

weavr has four operating modes:

| Mode | Flag | Description |
|------|------|-------------|
| TUI (default) | — | Interactive three-pane merge resolver |
| Headless | `--headless` | Automatic resolution, no UI |
| List | `--list` | Print conflicted files and exit |
| Check | `--check` | Check for conflicts and exit (no resolution) |

## Global flags

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--vcs` | `auto\|git\|jj` | `auto` | VCS backend (auto-detects; tries jj first) |
| `--config` | `PATH` | — | Configuration file path |
| `--theme` | `THEME` | — | Theme name (overrides config) |
| `--format` | `text\|json` | `text` | Output format |
| `--auto-stage` | flag | `false` | Stage resolved files after writing |
| `--no-stage` | flag | `false` | Disable staging entirely (no auto-stage, no prompt) |

### Headless mode flags

These flags require `--headless`:

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--strategy` | `left\|right\|both\|ast\|ai` | — | Resolution strategy |
| `--dedupe` | flag | `false` | Deduplicate for accept-both |
| `--dry-run` | flag | `false` | Print result without writing |
| `--fallback-strategy` | `left\|right\|both` | — | Fallback when AI declines/errors |
| `--fail-on-ambiguous` | flag | `false` | Exit 1 if any hunk can't be auto-resolved |

### Check mode flags

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--quiet` | flag | `false` | Suppress output (exit code only); requires `--check` |

## Subcommands

### `weavr init`

Initialize weavr in the current repository.

```bash
weavr init [OPTIONS]
```

| Flag | Default | Description |
|------|---------|-------------|
| `--force` | — | Overwrite existing config files |
| `--no-git` | — | Skip Git merge driver setup |
| `--global` | — | Configure merge driver in `~/.gitconfig` (conflicts with `--no-git`) |
| `--patterns` | `*.rs` | File patterns for `.gitattributes` (comma-separated) |
| `--no-jj` | — | Skip jj merge tool setup (requires `jj` feature) |
| `--jj-scope` | `repo` | Scope for jj config: `repo` or `user` (requires `jj` feature) |

**What init does:**
1. Creates `.weavr.toml` with documented defaults
2. Sets `merge.weavr.driver` in `.git/config` (or `~/.gitconfig` with `--global`)
3. Adds patterns to `.gitattributes`
4. Configures jj merge tool (if `jj` feature is enabled)

### `weavr merge-driver`

Run as a Git merge driver. Typically invoked by Git, not directly.

```bash
weavr merge-driver [OPTIONS] <BASE> <OURS> <THEIRS> [MARKER_SIZE] [PATH]
```

| Positional | Git variable | Description |
|------------|-------------|-------------|
| `BASE` | `%O` | Base (ancestor) version |
| `OURS` | `%A` | Current version (result written here) |
| `THEIRS` | `%B` | Other version |
| `MARKER_SIZE` | `%L` | Conflict marker size |
| `PATH` | `%P` | Pathname of the merged file |

| Flag | Description |
|------|-------------|
| `--strategy` | Resolution strategy (overrides config) |
| `--output` | Write result to separate file instead of overwriting ours |
| `--fallback-strategy` | Fallback when AI declines/errors |
| `--format` | Output format (JSON written to stderr) |
| `--log-file` | Path to append JSON log output (requires `--format=json`) |

### `weavr inspect`

Dump structured conflict data as JSON.

```bash
weavr inspect <FILE...>
```

Outputs hunk IDs, content, and context for each conflict in the given files. Useful for scripted workflows paired with `weavr resolve`.

### `weavr resolve`

Apply per-hunk resolutions from a JSON map.

```bash
weavr resolve <FILE...> --resolutions <PATH>
```

| Flag | Description |
|------|-------------|
| `--resolutions` | Path to JSON resolutions file, or `-` for stdin (required) |
| `--dry-run` | Preview resolved output without writing |
| `--fail-on-ambiguous` | Error if any hunk has no resolution |
| `--dedupe` | Enable deduplication for `both` strategy |
| `--auto-stage` | Stage resolved files |
| `--vcs` | VCS backend |
| `--format` | Output format |

**Scripted workflow example:**

```bash
# 1. Inspect conflicts and get hunk IDs
weavr inspect src/lib.rs > conflicts.json

# 2. Build a resolution map (manually or programmatically)
# 3. Apply resolutions
weavr resolve src/lib.rs --resolutions decisions.json
```

## Exit codes

| Code | Meaning |
|------|---------|
| `0` | Success (all conflicts resolved) |
| `1` | Unresolved conflicts remain |
| `2` | Error (parse failure, IO error, etc.) |
