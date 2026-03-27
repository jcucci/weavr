# Installation & Quick Start

## Prerequisites

- Rust 1.75 or later (install via [rustup](https://rustup.rs/))

## Install from source

```bash
cargo install weavr-cli
```

### With optional features

```bash
# AI-assisted resolution (Claude)
cargo install weavr-cli --features ai-claude

# AST-aware merging for Rust and TypeScript
cargo install weavr-cli --features ast-rust,ast-typescript

# Everything
cargo install weavr-cli --features ai-all,ast-all
```

See the [feature flags table](../README.md#feature-flags) for all available flags.

## Quick start

### 1. Register the merge driver

```bash
weavr init
```

This creates a `.weavr.toml` config file and registers weavr as a Git merge driver (and optionally as a jj merge tool). See [VCS integration](vcs.md) for details.

### 2. Trigger a merge conflict

```bash
git merge feature-branch
# or
jj new main feature-branch
```

### 3. Resolve conflicts

```bash
# Open the TUI
weavr

# Or resolve headlessly
weavr --headless --strategy left
```

In the TUI, navigate hunks with `j`/`k`, resolve with `o` (ours), `t` (theirs), or `b` (both), then save and quit with `:wq`. See [TUI usage](tui.md) for the full keybinding reference.

### 4. Done

weavr writes the resolved content back to the conflicted files. If `auto_stage` is enabled (or you confirm at the prompt), resolved files are staged automatically.

## Exit codes

| Code | Meaning |
|------|---------|
| `0` | All conflicts resolved successfully |
| `1` | Unresolved conflicts remain |
| `2` | Error (parse failure, IO error, etc.) |

## Platform notes

weavr builds on Linux, macOS, and Windows. The jj feature (enabled by default) requires the `jj` CLI to be installed separately for jj-specific workflows.
