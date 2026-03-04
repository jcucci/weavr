# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **Core merge engine** — Structured conflict model with explicit domain types, deterministic resolution, and full round-trip fidelity
- **Conflict parser** — Git conflict marker detection and parsing into structured `Conflict` objects
- **Resolution strategies** — Accept left, accept right, accept both (with ordering options), and manual resolution helpers
- **Merge session lifecycle** — State machine for merge sessions with lifecycle transitions
- **HunkContext** — Surrounding context for conflict hunks to improve resolution quality
- **Error types** — Comprehensive, typed error hierarchy across all crates
- **Terminal UI** — Ratatui-based TUI with Vim-style keybindings, side-by-side diff view, and inline conflict editing
- **Theme system** — 19 built-in themes including Catppuccin variants
- **Word-level diff highlighting** — Toggle between line-level and word-level diff rendering
- **Help system** — In-app keybinding reference and help overlay
- **Multi-file workflow** — Navigate between conflicted files with `:n`, `:prev`, `:files`
- **Git integration** — Repository discovery, conflict detection, and staging integration (`:wa`, `:wq`, `--auto-stage`)
- **AI provider support** — Pluggable AI providers (Claude, OpenAI, local) for merge explanations with caching
- **AST-based merging** — Structural merge strategies for Rust (syn), C# (tree-sitter), TypeScript (tree-sitter), and Go (tree-sitter)
- **Shared merger infrastructure** — Common confidence scoring, test utilities, and merger traits for AST strategies
- **CLI orchestration** — Feature-gated CLI with headless mode, configuration via TOML, and XDG directory support
- **CI pipeline** — GitHub Actions for format, clippy, test, doc, and cross-platform checks
