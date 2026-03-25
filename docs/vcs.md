# VCS Integration

weavr integrates with Git and Jujutsu (jj) as a merge driver / merge tool.

## VCS detection

By default, weavr auto-detects your VCS (trying jj first, then Git). Override with:

```bash
weavr --vcs git    # Force Git
weavr --vcs jj     # Force jj
weavr --vcs auto   # Auto-detect (default)
```

## Git setup

### Using `weavr init`

```bash
weavr init
```

This does three things:

1. **Creates `.weavr.toml`** — project config with documented defaults
2. **Configures merge driver** — sets `merge.weavr.driver` in `.git/config`:
   ```
   [merge "weavr"]
       name = weavr merge driver
       driver = weavr merge-driver %O %A %B %L %P
   ```
3. **Adds `.gitattributes` patterns** — tells Git which files use the weavr driver:
   ```
   *.rs merge=weavr
   ```

#### Options

```bash
# Custom file patterns
weavr init --patterns "*.rs,*.ts,*.go"

# Global config (~/.gitconfig) instead of repo-local
weavr init --global

# Skip Git setup entirely
weavr init --no-git

# Overwrite existing config
weavr init --force
```

### Manual setup

If you prefer to configure Git manually:

```bash
# Register the merge driver
git config merge.weavr.name "weavr merge driver"
git config merge.weavr.driver "weavr merge-driver %O %A %B %L %P"
```

Add to `.gitattributes`:

```
*.rs merge=weavr
*.ts merge=weavr
```

### Merge driver

When Git encounters a conflict in a file matching the pattern, it invokes:

```
weavr merge-driver %O %A %B %L %P
```

| Variable | Meaning |
|----------|---------|
| `%O` | Base (ancestor) version |
| `%A` | Current (ours) version — result written here |
| `%B` | Other (theirs) version |
| `%L` | Conflict marker size |
| `%P` | File pathname |

See [CLI reference](cli.md#weavr-merge-driver) for additional merge-driver flags.

## Auto-staging

After resolving conflicts, weavr can automatically stage the resolved files:

```bash
# CLI flag
weavr --auto-stage

# Or disable staging entirely
weavr --no-stage
```

**Config:**

```toml
[git]
auto_stage = false    # Auto-stage without prompting
stage_prompt = true   # Prompt to stage (default)
```

When `auto_stage` is false and `stage_prompt` is true (the default), weavr asks whether to stage after resolution completes.

## Jujutsu (jj) setup

*Requires the `jj` feature (enabled by default).*

### Using `weavr init`

```bash
weavr init
```

When the `jj` feature is enabled, `weavr init` also configures jj's merge tool settings.

#### Options

```bash
# Skip jj setup
weavr init --no-jj

# User-level jj config instead of repo-level
weavr init --jj-scope user
```

### Configuration

```toml
[jj]
squash_after_resolve = false
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `squash_after_resolve` | bool | `false` | Run `jj squash` after all hunks in a file are resolved |
