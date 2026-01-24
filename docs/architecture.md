# Architecture

This document describes the high-level architecture of weavr, including crate structure, dependency direction, and extensibility points.

---

## Design Philosophy

weavr follows a **library-first architecture**:

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
weavr/
├── crates/
│   ├── weavr-core/     # Pure merge logic (no IO, no UI)
│   ├── weavr-cli/      # CLI orchestration + headless mode
│   ├── weavr-tui/      # Terminal UI (ratatui)
│   ├── weavr-git/      # Git integration
│   ├── weavr-ast/      # Language-aware merging
│   └── weavr-ai/       # AI provider integrations
```

---

## Crate Responsibilities

### weavr-core

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

### weavr-cli

Command-line interface and orchestration:

- Argument parsing (clap)
- File discovery
- Headless mode execution
- Exit code handling
- Configuration loading

```
weavr              # open all conflicted files
weavr file.rs      # open specific file
weavr --headless   # auto-apply rules
```

### weavr-tui

Terminal user interface (ratatui):

- Three-pane layout (left, right, result)
- Hunk navigation
- Resolution selection
- Keyboard bindings
- Theming

**Key principle:** The TUI is a thin wrapper. It displays state and captures input but never performs merge logic.

### weavr-git

Git integration:

- Detect conflicted files via `git status`
- Read conflict markers
- Stage resolved files
- Respect `.gitattributes`

### weavr-ast

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

### weavr-ai

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
                    │ weavr-core  │
                    └──────┬──────┘
                           │
         ┌─────────────────┼─────────────────┐
         │                 │                 │
         ▼                 ▼                 ▼
   ┌──────────┐      ┌──────────┐      ┌──────────┐
   │ weavr-cli│      │ weavr-tui│      │ weavr-git│
   └──────────┘      └──────────┘      └──────────┘
         │
         ▼
   ┌──────────┐      ┌──────────┐
   │ weavr-ast│      │ weavr-ai │
   └──────────┘      └──────────┘
```

**Rules:**
- `weavr-core` has **no dependencies** on other weavr crates
- All other crates depend on `weavr-core`
- `weavr-ast` and `weavr-ai` are optional features
- No dependency cycles

---

## Extensibility Points

### Resolution Strategies

New strategies implement the `ResolutionStrategy` trait:

```rust
pub trait ResolutionStrategy {
    fn propose(&self, hunk: &ConflictHunk) -> Option<Resolution>;
    fn kind(&self) -> ResolutionStrategyKind;
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
| `ast-rust` | weavr-ast | Rust AST merging |
| `ast-csharp` | weavr-ast | C# AST merging |
| `ast-typescript` | weavr-ast | TypeScript AST merging |
| `ast-go` | weavr-ast | Go AST merging |
| `ai-claude` | weavr-ai | Claude provider |
| `ai-openai` | weavr-ai | OpenAI provider |
| `ai-local` | weavr-ai | Local LLM support |

---

## Theming

Theming is a first-class concern in `weavr-tui`:

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
~/.config/weavr/config.toml
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

The following are explicitly **not** goals for weavr:

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
- `weavr-core`: Pure function tests, golden file tests
- Determinism tests (same input → same output)

### Integration Tests
- `weavr-cli`: End-to-end merge scenarios
- `weavr-git`: Git repository fixtures

### Property Tests
- Resolution application is reversible
- Validation catches all markers

### UI Tests
- `weavr-tui`: Snapshot tests for rendering
