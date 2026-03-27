# Contributing

Contributions are welcome! For large changes, please open an issue to discuss before starting work.

## Development setup

### Prerequisites

- Rust 1.75+ (install via [rustup](https://rustup.rs/))

### Build and test

```bash
# Clone
git clone https://github.com/jcucci/weavr.git
cd weavr

# Build all crates
cargo build --workspace

# Run tests
cargo test --workspace

# Run clippy (must match CI)
cargo clippy --workspace --all-targets -- -D warnings

# Check formatting
cargo fmt --all --check
```

### Testing with optional features

```bash
cargo test --workspace --features ai-all,ast-all
```

### Building docs

```bash
cargo doc --workspace --no-deps
```

## Workspace structure

| Crate        | Purpose                                                   |
| ------------ | --------------------------------------------------------- |
| `weavr-core` | Pure merge logic and domain model (no I/O, no Git, no UI) |
| `weavr-cli`  | CLI entry point and headless execution                    |
| `weavr-tui`  | Terminal UI (ratatui)                                     |
| `weavr-git`  | Git integration                                           |
| `weavr-vcs`  | VCS abstraction layer                                     |
| `weavr-jj`   | Jujutsu (jj) integration                                  |
| `weavr-ai`   | AI provider integrations                                  |
| `weavr-ast`  | AST-based merge (tree-sitter, syn)                        |

## Golden rules

1. **weavr-core must remain pure** — no filesystem, no Git, no UI, no network
2. **No hidden decisions** — all merge resolutions must be explicit
3. **Determinism is mandatory** — same inputs + same decisions = same output
4. **Graceful fallback** — AST/structured merging always falls back to text
5. **Terminal-first** — keyboard-driven workflows are the primary UX

## Code style

- Rust edition 2021
- `unsafe_code = "forbid"` (workspace-wide)
- Clippy pedantic warnings enabled
- Always run clippy with `--all-targets` to match CI

## PR guidelines

- Prefer extending the domain model over adding flags or special cases
- New domain types that only depend on core types belong in `weavr-core`
- Public APIs should be designed before implementation
- Keep PRs focused — one concern per PR when possible
