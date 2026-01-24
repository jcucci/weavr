# Public API — weavr-core

This document defines the intended public API surface of `weavr-core`. This API is considered **locked** and changes require careful consideration.

---

## Design Principles

The `weavr-core` API is designed to be:

- **Pure** — No filesystem, no Git, no network, no UI
- **Deterministic** — Same inputs + same decisions = same output
- **Explicit** — No hidden mutation or global state
- **Testable** — All operations are unit-testable

---

## Stability Policy

All public types and methods documented on this page are considered **stable** and follow [Semantic Versioning](https://semver.org/):

- **Breaking changes** require a major version bump
- **New features** may be added in minor versions
- **Bug fixes** may be added in patch versions

For the full stability policy including what constitutes a breaking change, see the [weavr-core README](../crates/weavr-core/README.md#api-stability-policy).

---

## Primary Type

```rust
pub struct MergeSession {
    input: MergeInput,
    hunks: Vec<ConflictHunk>,
    state: MergeState,
    resolutions: HashMap<HunkId, Resolution>,
}
```

---

## Construction

### From MergeInput

```rust
impl MergeSession {
    pub fn new(input: MergeInput) -> Result<Self, ParseError>
}
```

Creates a new session and parses conflict markers.

### From Raw Content

```rust
impl MergeSession {
    pub fn from_content(
        left: &str,
        right: &str,
        base: Option<&str>,
    ) -> Result<Self, ParseError>
}
```

Convenience constructor for testing and simple use cases.

---

## Inspection

### Get Hunks

```rust
impl MergeSession {
    pub fn hunks(&self) -> &[ConflictHunk]
}
```

Returns all conflict hunks in file order.

### Get Specific Hunk

```rust
impl MergeSession {
    pub fn hunk(&self, id: HunkId) -> Option<&ConflictHunk>
}
```

### Get State

```rust
impl MergeSession {
    pub fn state(&self) -> MergeState
}
```

Returns the current session state.

### Check Resolution Status

```rust
impl MergeSession {
    pub fn is_fully_resolved(&self) -> bool
}
```

Returns `true` if all hunks have resolutions.

### Get Unresolved Hunks

```rust
impl MergeSession {
    pub fn unresolved_hunks(&self) -> Vec<HunkId>
}
```

---

## Resolution Operations

### Propose Resolutions

```rust
impl MergeSession {
    pub fn propose_resolutions(
        &self,
        hunk_id: HunkId,
        strategies: &[&dyn ResolutionStrategy],
    ) -> Vec<Resolution>
}
```

Generates candidate resolutions without modifying state. Strategies implement the `ResolutionStrategy` trait; each returned `Resolution` contains a `ResolutionStrategyKind` describing how it was generated.

### Set Resolution

```rust
impl MergeSession {
    pub fn set_resolution(
        &mut self,
        hunk_id: HunkId,
        resolution: Resolution,
    ) -> Result<(), ResolutionError>
}
```

Applies a resolution to a hunk.

### Clear Resolution

```rust
impl MergeSession {
    pub fn clear_resolution(
        &mut self,
        hunk_id: HunkId,
    ) -> Result<(), ResolutionError>
}
```

Removes a resolution, returning hunk to `Unresolved` state.

### Get Resolution

```rust
impl MergeSession {
    pub fn resolution(&self, hunk_id: HunkId) -> Option<&Resolution>
}
```

---

## Validation and Completion

### Validate

```rust
impl MergeSession {
    pub fn validate(&self) -> Result<(), ValidationError>
}
```

Checks that:
- All hunks are resolved
- No conflict markers remain in output
- (Optional) Output is syntactically valid

### Apply Resolutions

```rust
impl MergeSession {
    pub fn apply(&self) -> Result<String, ApplyError>
}
```

Generates the merged output text. Does not consume the session.

### Complete

```rust
impl MergeSession {
    pub fn complete(self) -> Result<MergeResult, CompletionError>
}
```

Finalizes the session and returns the immutable result. Consumes the session.

---

## Hunk Operations

### ConflictHunk Methods

```rust
impl ConflictHunk {
    pub fn id(&self) -> HunkId
    pub fn left(&self) -> &HunkContent
    pub fn right(&self) -> &HunkContent
    pub fn base(&self) -> Option<&HunkContent>
    pub fn context(&self) -> &HunkContext
    pub fn state(&self) -> HunkState
    pub fn is_resolved(&self) -> bool
}
```

---

## Type Definitions

### MergeInput

```rust
pub struct MergeInput {
    pub left: FileVersion,
    pub right: FileVersion,
    pub base: Option<FileVersion>,
}
```

### FileVersion

```rust
pub struct FileVersion {
    pub path: PathBuf,
    pub content: String,
}
```

### MergeResult

```rust
pub struct MergeResult {
    pub content: String,
    pub unresolved_hunks: Vec<HunkId>,
    pub warnings: Vec<MergeWarning>,
    pub summary: MergeSummary,
}
```

### Resolution

```rust
pub struct Resolution {
    pub kind: ResolutionStrategyKind,
    pub content: String,
    pub metadata: ResolutionMetadata,
}
```

---

## Error Types

```rust
pub enum ParseError {
    InvalidMarkers(String),
    MalformedContent(String),
}

pub enum ResolutionError {
    HunkNotFound(HunkId),
    InvalidResolution(String),
}

pub enum ValidationError {
    UnresolvedHunks(Vec<HunkId>),
    MarkersRemain(usize),
    SyntaxError(String),
}

pub enum ApplyError {
    NotFullyResolved,
    InternalError(String),
}

pub enum CompletionError {
    ValidationFailed(ValidationError),
    ApplyFailed(ApplyError),
}
```

---

## API Guarantees

| Guarantee | Description |
|-----------|-------------|
| No IO | Core never reads/writes files |
| No hidden mutation | All state changes are explicit method calls |
| No global state | All state is contained in `MergeSession` |
| Deterministic | Same inputs + decisions = same output |
| Thread-safe | `MergeSession` can be `Send` (not `Sync`) |

---

## What Core Does NOT Do

The following are explicitly **not** the responsibility of `weavr-core`:

| Responsibility | Owner |
|----------------|-------|
| Read files from disk | `weavr-cli` / `weavr-git` |
| Write files to disk | `weavr-cli` |
| Invoke Git commands | `weavr-git` |
| Render UI | `weavr-tui` |
| Call AI providers | `weavr-ai` |
| Parse AST | `weavr-ast` |

This separation ensures core remains pure and testable.

---

## Example Usage

```rust
use weavr_core::{
    MergeSession, MergeInput, Resolution, ResolutionStrategyKind,
    AcceptBothStrategy, AcceptBothOptions, BothOrder,
};

// Create session
let input = MergeInput { left, right, base: Some(base) };
let mut session = MergeSession::new(input)?;

// Inspect hunks
for hunk in session.hunks() {
    println!("Hunk {}: {:?}", hunk.id(), hunk.state());
}

// Get proposals using strategy trait
let strategy = AcceptBothStrategy {
    options: AcceptBothOptions {
        order: BothOrder::LeftThenRight,
        deduplicate: true,
        trim_whitespace: false,
    },
};
let proposals = session.propose_resolutions(hunk_id, &[&strategy]);

// Apply resolution (using ResolutionStrategyKind enum)
session.set_resolution(
    hunk_id,
    Resolution {
        kind: ResolutionStrategyKind::AcceptBoth(options),
        content: merged_content,
        metadata: ResolutionMetadata::default(),
    },
)?;

// Validate and complete
session.validate()?;
let result = session.complete()?;

println!("Merged content:\n{}", result.content);
```
