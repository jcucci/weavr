# Architecture

This document describes the high-level architecture of meldr, including crate structure, dependency direction, and extensibility points.

---

## Design Philosophy

meldr follows a **library-first architecture**:

- The core merge logic is a pure library
- The TUI is one of many possible frontends
- Headless execution uses the same engine as interactive mode

This enables:
- Neovim integration
- CI/CD automation
- Editor plugins
- Testing without UI

---

## Crate Structure

```
meldr/
├── crates/
│   ├── meldr-core/     # Pure merge logic (no IO, no UI)
│   ├── meldr-cli/      # CLI orchestration + headless mode
│   ├── meldr-tui/      # Terminal UI (ratatui)
│   ├── meldr-git/      # Git integration
│   ├── meldr-ast/      # Language-aware merging
│   └── meldr-ai/       # AI provider integrations
```

---

## Crate Responsibilities

### meldr-core

The heart of the system. Contains all "hard thinking":

- Parsing conflict markers
- Representing conflict hunks
- Resolution strategies
- Resolution application
- Validation

**Explicitly does NOT:**
- Read/write files
- Know about Git
- Render UI
- Call AI providers
- Parse language ASTs

This separation is **non-negotiable**.

### meldr-cli

Command-line interface and orchestration:

- Argument parsing (clap)
- File discovery
- Headless mode execution
- Exit code handling
- Configuration loading

```
meldr              # open all conflicted files
meldr file.rs      # open specific file
meldr --headless   # auto-apply rules
```

### meldr-tui

Terminal user interface (ratatui):

- Three-pane layout (left, right, result)
- Hunk navigation
- Resolution selection
- Keyboard bindings
- Theming

**Key principle:** The TUI is a thin wrapper. It displays state and captures input but never performs merge logic.

### meldr-git

Git integration:

- Detect conflicted files via `git status`
- Read conflict markers
- Stage resolved files
- Respect `.gitattributes`

### meldr-ast

Language-aware merging:

- Parse AST fragments
- Structural merge strategies
- Formatter integration
- Language detection

Supports (planned):
- Rust
- C#
- TypeScript
- Go

### meldr-ai

AI provider integrations:

- Provider abstraction
- Async suggestion fetching
- Confidence scoring
- Rate limiting

Providers (planned):
- Claude
- OpenAI
- Local LLMs

---

## Dependency Direction

```
                    ┌─────────────┐
                    │ meldr-core  │
                    └──────┬──────┘
                           │
         ┌─────────────────┼─────────────────┐
         │                 │                 │
         ▼                 ▼                 ▼
   ┌──────────┐      ┌──────────┐      ┌──────────┐
   │ meldr-cli│      │ meldr-tui│      │ meldr-git│
   └──────────┘      └──────────┘      └──────────┘
         │
         ▼
   ┌──────────┐      ┌──────────┐
   │ meldr-ast│      │ meldr-ai │
   └──────────┘      └──────────┘
```

**Rules:**
- `meldr-core` has **no dependencies** on other meldr crates
- All other crates depend on `meldr-core`
- `meldr-ast` and `meldr-ai` are optional features
- No dependency cycles

---

## Extensibility Points

### Resolution Strategies

New strategies implement the `ResolutionStrategy` trait:

```rust
pub trait ResolutionStrategy {
    fn propose(&self, hunk: &ConflictHunk) -> Option<Resolution>;
}
```

### Validators

Custom validation can be added:

```rust
pub trait Validator {
    fn validate(&self, output: &str) -> Result<(), ValidationError>;
}
```

### AI Providers

New AI providers implement:

```rust
pub trait AiProvider {
    async fn suggest(&self, hunk: &ConflictHunk) -> Option<Resolution>;
    async fn explain(&self, hunk: &ConflictHunk) -> Option<String>;
}
```

### AST Languages

New languages implement:

```rust
pub trait AstMerger {
    fn supports(&self, path: &Path) -> bool;
    fn try_merge(&self, hunk: &ConflictHunk) -> Option<MergedResult>;
}
```

---

## Feature Flags

Optional functionality is gated behind Cargo features:

| Feature | Crate | Description |
|---------|-------|-------------|
| `ast-rust` | meldr-ast | Rust AST merging |
| `ast-csharp` | meldr-ast | C# AST merging |
| `ast-typescript` | meldr-ast | TypeScript AST merging |
| `ast-go` | meldr-ast | Go AST merging |
| `ai-claude` | meldr-ai | Claude provider |
| `ai-openai` | meldr-ai | OpenAI provider |
| `ai-local` | meldr-ai | Local LLM support |

---

## Theming

Theming is a first-class concern in `meldr-tui`:

```rust
pub struct Theme {
    pub base: ColorPalette,
    pub conflict: ConflictColors,
    pub diff: DiffColors,
    pub ui: UiColors,
}

pub struct ConflictColors {
    pub left: Style,
    pub right: Style,
    pub both: Style,
    pub unresolved: Style,
}
```

**Principles:**
- Semantic colors, not "red/green"
- No hard-coded ANSI values
- External configuration (TOML)
- Runtime-switchable
- Dark + light themes built-in

---

## Configuration

Configuration follows XDG conventions:

```
~/.config/meldr/config.toml
```

Example:

```toml
[theme]
name = "dark"

[strategies]
default = "accept-both"
prefer_ast = true

[ai]
enabled = false
provider = "claude"

[headless]
fail_on_ambiguous = true
```

---

## Non-Goals

The following are explicitly **not** goals for meldr:

| Non-Goal | Reason |
|----------|--------|
| GUI application | Terminal-first philosophy |
| Web interface | Out of scope |
| Real-time collaboration | Complexity vs. value |
| Language server protocol | May reconsider later |
| Automatic conflict resolution | Violates "no hidden decisions" |

---

## Testing Strategy

### Unit Tests
- `meldr-core`: Pure function tests, golden file tests
- Determinism tests (same input → same output)

### Integration Tests
- `meldr-cli`: End-to-end merge scenarios
- `meldr-git`: Git repository fixtures

### Property Tests
- Resolution application is reversible
- Validation catches all markers

### UI Tests
- `meldr-tui`: Snapshot tests for rendering
