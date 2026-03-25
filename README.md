# weavr

A terminal-first merge conflict resolver that treats conflicts as structured data, not raw text.

weavr models each conflict hunk as an explicit domain object with typed resolution strategies, undo/redo, and deterministic output. It ships with a keyboard-driven TUI, a headless CLI for scripting and CI, and optional AI and AST-based merge assistance.

<!-- TODO: terminal screenshot / recording -->

## Why weavr?

- **Structured conflicts** — hunks are parsed into a domain model, not string-matched
- **Explicit resolutions** — every decision is visible, reversible, and replayable
- **Assistive, not opaque** — AI and AST suggestions are opt-in and never auto-apply
- **Terminal-first** — full keyboard-driven TUI with Vim-style bindings
- **Git and jj** — works as a merge driver for both Git and Jujutsu

## Quick start

```bash
# Install (Rust 1.75+)
cargo install weavr-cli

# Register as your Git merge driver
weavr init

# After a merge conflict, open the TUI
weavr
```

See [Installation](docs/installation.md) for feature flags and platform notes.

## Features

| Feature | Description | Docs |
|---------|-------------|------|
| TUI | Three-pane merge view, inline editing, syntax highlighting | [TUI guide](docs/tui.md) |
| Headless mode | Scriptable conflict resolution for CI pipelines | [CLI reference](docs/cli.md) |
| AI suggestions | Per-hunk suggestions from Claude, OpenAI, or local models | [AI integration](docs/ai.md) |
| AST merge | Language-aware structural merging (Rust, C#, TypeScript, Go) | [AST merge](docs/ast-merge.md) |
| Themes | 19 built-in themes + custom theme support | [Themes](docs/themes.md) |
| Configuration | Layered TOML config with per-project overrides | [Configuration](docs/configuration.md) |
| Git + jj | Merge driver integration for Git and Jujutsu | [VCS integration](docs/vcs.md) |

## Feature flags

weavr uses Cargo feature flags for optional integrations:

| Flag | Description |
|------|-------------|
| `jj` | Jujutsu VCS support *(default)* |
| `ai-claude` | Claude AI provider |
| `ai-openai` | OpenAI provider |
| `ai-local` | Local model support (Ollama) |
| `ai-all` | All AI providers |
| `ast-rust` | Rust AST merge (syn) |
| `ast-csharp` | C# AST merge (tree-sitter) |
| `ast-typescript` | TypeScript AST merge (tree-sitter) |
| `ast-go` | Go AST merge (tree-sitter) |
| `ast-all` | All AST languages |

```bash
# Install with Claude AI and Rust AST support
cargo install weavr-cli --features ai-claude,ast-rust
```

## Documentation

- [Installation & quick start](docs/installation.md)
- [CLI reference](docs/cli.md)
- [TUI usage & keybindings](docs/tui.md)
- [Configuration](docs/configuration.md)
- [Themes & customization](docs/themes.md)
- [AI integration](docs/ai.md)
- [AST merge](docs/ast-merge.md)
- [VCS integration](docs/vcs.md)

## License

Licensed under either of:
- Apache License, Version 2.0
- MIT license

at your option.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup and guidelines.
