# Merge Lifecycle

This document defines the lifecycle of a merge operation in weavr.

---

## Overview

A conflicted file moves through distinct phases:

```
Uninitialized → Parsed → Active → FullyResolved → Applied → Validated → Completed
```

The lifecycle supports:
- Pausing and resuming
- Rolling back decisions
- Re-evaluating strategies
- Headless automation

---

## MergeSession

A `MergeSession` represents one attempt to merge a single file.

```rust
pub struct MergeSession {
    pub input: MergeInput,
    pub hunks: Vec<ConflictHunk>,
    pub state: MergeState,
    pub resolutions: HashMap<HunkId, Resolution>,
}
```

---

## MergeState

```rust
pub enum MergeState {
    Uninitialized,
    Parsed,
    Active,
    FullyResolved,
    Applied,
    Validated,
    Completed,
}
```

### State Descriptions

| State | Description |
|-------|-------------|
| `Uninitialized` | Raw MergeInput provided, no parsing performed |
| `Parsed` | Conflict markers parsed, hunks identified, no resolutions chosen |
| `Active` | At least one hunk unresolved, user or automation selecting resolutions |
| `FullyResolved` | All hunks have a Resolution, no output generated yet |
| `Applied` | Resolutions applied, output text produced |
| `Validated` | Output contains no conflict markers, file is valid |
| `Completed` | Final MergeResult generated |

### State Transitions

```
Uninitialized
      ↓ parse()
   Parsed
      ↓ begin_resolution()
   Active
      ↓ (all hunks resolved)
FullyResolved
      ↓ apply()
   Applied
      ↓ validate()
  Validated
      ↓ complete()
  Completed
```

---

## Hunk-Level States

Each hunk has its own lifecycle, independent of other hunks.

```rust
pub enum HunkState {
    Unresolved,
    Proposed(Vec<Resolution>),
    Resolved(Resolution),
    Invalid,
}
```

### State Descriptions

| State | Description |
|-------|-------------|
| `Unresolved` | No resolution chosen (default state after parsing) |
| `Proposed` | One or more candidate resolutions available (source-tagged: AST/AI/heuristic) |
| `Resolved` | Exactly one resolution selected (may be overridden) |
| `Invalid` | Resolution chosen but rejected (syntax error, empty output, validation failure) |

### Hunk State Transitions

```
Unresolved
     ↓ (strategies run)
  Proposed
     ↓ (user or headless choice)
  Resolved
     ↓ (validation failure)
   Invalid
     ↺ (new resolution)
  Resolved
```

---

## Resolution Lifecycle

A Resolution is **inert** until applied. This distinction enables:
- Replay
- Strategy changes
- "Try AST merge → revert → accept both"

```
Resolution (decision)
        ↓
   Application
        ↓
  Resulting text
```

---

## Core Transitions (API)

These are the only allowed transitions in `weavr-core`.

### Parsing

```rust
fn parse(contents: &str, input: MergeInput) -> Result<MergeSession, ParseError>
```

- Creates hunks from conflict markers
- Sets `MergeState::Parsed`
- All hunks start as `Unresolved`

### Proposing Resolutions

```rust
fn propose_resolutions(
    hunk: &ConflictHunk,
    strategies: &[&dyn ResolutionStrategy],
) -> Vec<Resolution>
```

- Does not mutate state
- Can be called repeatedly
- Order matters (priority)
- Result: `Unresolved → Proposed`

### Selecting a Resolution

```rust
fn set_resolution(
    hunk_id: HunkId,
    resolution: Resolution,
)
```

Valid transitions:
- `Proposed → Resolved`
- `Unresolved → Resolved` (manual selection)
- `Resolved → Resolved` (override)
- `Invalid → Resolved` (retry)

### Clearing a Resolution

```rust
fn clear_resolution(hunk_id: HunkId)
```

- Result: `Resolved → Unresolved`
- Enables undo/retry

### Applying Resolutions

```rust
impl MergeSession {
    fn apply(&mut self) -> String
}
```

- Produces output text
- Result: `FullyResolved → Applied`

### Validation

```rust
fn validate(output: &str) -> Result<(), ValidationError>
```

Checks:
- No conflict markers remain
- Optionally: syntax validity (language-aware)

### Completion

```rust
fn complete(session: MergeSession) -> MergeResult
```

- Finalizes the merge
- Returns immutable `MergeResult`

---

## Invalid Transitions

The following transitions are not allowed:

| Invalid Transition | Reason |
|--------------------|--------|
| Apply resolution before parsing | No hunks exist |
| Complete without validation | May contain markers |
| Validate with unresolved hunks | Incomplete merge |
| Parse already-parsed file | State corruption |

---

## Error States

Errors are **values**, not panics.

| Error | Description |
|-------|-------------|
| `ParseError` | Failed to parse conflict markers |
| `ResolutionError` | Invalid resolution for hunk |
| `ValidationError` | Output failed validation checks |

---

## Headless Mode

With this lifecycle, headless mode is straightforward:

```rust
let plan = MergePlan::default()
    .with_strategy(AstFirst)
    .with_fallback(TextAcceptBoth);

engine.run(plan)?;
```

Exit codes:
- `0`: Fully resolved
- `1`: Unresolved conflicts remain
- `2`: Error
