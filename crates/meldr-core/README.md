# meldr-core

Pure merge logic engine for meldr.

This crate contains the core domain model and merge logic. It is intentionally pure and has no dependencies on:

- Filesystem operations
- Git commands
- UI/terminal rendering
- Network/AI providers

All merge decisions are explicit and deterministic.

## API Stability Policy

This crate follows [Semantic Versioning 2.0.0](https://semver.org/). The public API is organized into stability tiers:

### Stability Tiers

| Tier | Guarantees | Markers |
|------|------------|---------|
| **Stable** | Breaking changes require major version bump | Default for public items |
| **Unstable** | May change in minor versions | `#[cfg(feature = "...")]` or `#[doc(hidden)]` |
| **Internal** | Not part of public API | Private modules and `pub(crate)` items |

### What Constitutes a Breaking Change

The following changes to **stable** APIs require a major version bump:

- Removing public types, traits, or methods
- Changing method signatures (parameters, return types)
- Changing the behavior of existing methods in ways that break existing code
- Removing enum variants
- Adding required fields to structs (without defaults)
- Changing type bounds on generics
- Removing trait implementations

The following are **not** considered breaking changes:

- Adding new public types, traits, or methods
- Adding new enum variants (when marked `#[non_exhaustive]`)
- Adding optional fields to structs
- Adding new trait implementations
- Performance improvements
- Bug fixes (even if they change observable behavior that was clearly incorrect)

### Feature Gating Strategy

Experimental or unstable features are gated behind Cargo features:

```toml
[dependencies]
meldr-core = { version = "0.1", features = ["experimental-feature"] }
```

Unstable features:
- May change or be removed in minor versions
- Are documented with stability warnings
- Graduate to stable after proving their design

### Current Stability Status

All public types in meldr-core 0.1.x are considered **stable**:

- Error types: `ParseError`, `ResolutionError`, `ValidationError`, `ApplyError`, `CompletionError`
- Hunk types: `HunkId`, `HunkContent`, `HunkContext`, `HunkState`, `ConflictHunk`
- Input types: `FileVersion`, `MergeInput`
- Resolution types: `Resolution`, `ResolutionStrategyKind`, `ResolutionSource`, `ResolutionMetadata`, `BothOrder`, `AcceptBothOptions`
- Result types: `MergeResult`, `MergeSummary`, `MergeWarning`
- Session types: `MergeSession`, `MergeState`

## Usage

```rust
use meldr_core::{MergeSession, MergeInput, FileVersion, Resolution, ResolutionStrategyKind};

// Create input from file versions
let input = MergeInput {
    left: FileVersion { path: "file.rs".into(), content: left_content },
    right: FileVersion { path: "file.rs".into(), content: right_content },
    base: Some(FileVersion { path: "file.rs".into(), content: base_content }),
};

// Create a merge session
let mut session = MergeSession::new(input)?;

// Inspect conflicts
for hunk in session.hunks() {
    println!("Conflict at lines {}-{}", hunk.context.start_line_left, hunk.context.start_line_right);
}

// Resolve conflicts
let resolution = Resolution {
    kind: ResolutionStrategyKind::AcceptLeft,
    content: session.hunks()[0].left.text.clone(),
    metadata: Default::default(),
};
session.set_resolution(session.hunks()[0].id, resolution)?;

// Complete the merge
let result = session.complete()?;
```

## License

See the repository root for license information.
