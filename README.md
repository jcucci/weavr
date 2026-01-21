# meldr

**A terminal-first merge conflict resolver with structured, language-aware resolution.**

`meldr` is a modern, terminal-native tool for resolving Git merge conflicts safely and efficiently.
Instead of treating conflicts as raw text, `meldr` models them as structured data, enabling
powerful workflows such as first-class “accept both”, language-aware merging, and optional
AI-assisted suggestions.

`meldr` is designed for terminal-first developers, with a TUI, a headless CLI mode, and
editor integrations (starting with Neovim).

---

## Why meldr?

Merge conflicts are still one of the most frustrating parts of day-to-day development.
Most tools either:
- Treat conflicts as plain text, or
- Hide important decisions behind opaque automation

`meldr` takes a different approach:
- Conflicts are structured, not just text
- Every resolution is explicit and reversible
- Automation (AST or AI) assists but never decides

---

## Features

### Core
- Parse Git conflict markers into structured hunks
- Accept left / right / both (with configurable strategies)
- Manual per-hunk editing
- Deterministic, replayable merge decisions
- Headless (non-TUI) execution

### TUI
- Three-pane merge view (left / right / result)
- Keyboard-first navigation
- Configurable theming (dark / light)

### Language Awareness (Planned)
- AST-based merging for:
  - Rust
  - C#
  - TypeScript
  - Go
- Structural merging (imports, functions, declarations)
- Safe fallback to text-based merging

### AI Assistance (Planned, Optional)
- Per-hunk AI merge suggestions
- Difference explanations
- Fully opt-in and non-blocking

---

## Project Status

⚠️ **Early development**

The core domain model and merge engine are being implemented.
APIs may change until the first stable release.

---

## Building from Source

### Prerequisites

- Rust 1.75 or later (install via [rustup](https://rustup.rs/))

### Build

```bash
# Clone the repository
git clone https://github.com/jcucci/meldr.git
cd meldr

# Build all crates
cargo build

# Run tests
cargo test

# Run the CLI
cargo run --bin meldr -- --help

# Check formatting and lints
cargo fmt --check
cargo clippy --workspace
```

### Development

```bash
# Format code
cargo fmt

# Run clippy with auto-fix
cargo clippy --fix --workspace --allow-dirty

# Build documentation
cargo doc --workspace --open
```

---

## Architecture

`meldr` is built as a collection of small, focused crates:

- `meldr-core` — Pure merge engine and domain model
- `meldr-cli` — CLI and headless execution
- `meldr-tui` — Terminal UI
- (Planned) AST and AI integration crates

The core engine is UI- and Git-agnostic, making it easy to integrate `meldr` into editors,
CI workflows, or other tools.

---

## License

Licensed under either of:
- Apache License, Version 2.0
- MIT license

at your option.

---

## Contributing

Contributions are welcome, but the project is still settling its foundations.
If you’re interested in contributing, please open an issue to discuss ideas before
starting large changes.
