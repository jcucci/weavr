# Domain Model

This document defines the core domain objects used by `meldr-core`.

The domain model is:
- UI-agnostic
- Git-agnostic
- Deterministic
- Serializable

---

## MergeInput

Represents the raw inputs to a merge operation.

| Field | Type | Description |
|-------|------|-------------|
| `left` | `FileVersion` | The left (HEAD) version |
| `right` | `FileVersion` | The right (MERGE_HEAD) version |
| `base` | `Option<FileVersion>` | Optional base for 3-way merge |

- `left` and `right` are always required
- `base` is optional (to support 2-way merges)

---

## FileVersion

Represents a single version of a file.

| Field | Type | Description |
|-------|------|-------------|
| `path` | `PathBuf` | Path to the file |
| `content` | `String` | File contents |

The content is treated as opaque text at this level.

---

## MergeSession

Represents a single merge attempt for a file.

| Field | Type | Description |
|-------|------|-------------|
| `input` | `MergeInput` | The original merge inputs |
| `hunks` | `Vec<ConflictHunk>` | Parsed conflict regions |
| `state` | `MergeState` | Current session state |
| `resolutions` | `Map<HunkId, Resolution>` | Applied resolutions |

---

## MergeState

The state of a merge session.

```
Uninitialized → Parsed → Resolving → Validated → Completed
```

| State | Description |
|-------|-------------|
| `Uninitialized` | Raw MergeInput provided, no parsing performed |
| `Parsed` | Conflict markers parsed, hunks created |
| `Resolving` | User is applying resolutions |
| `Validated` | All hunks resolved, validation passed |
| `Completed` | Final MergeResult generated |

---

## ConflictHunk

Represents a contiguous region of conflicting content.

| Field | Type | Description |
|-------|------|-------------|
| `id` | `HunkId` | Unique identifier |
| `left` | `HunkContent` | Left side content |
| `right` | `HunkContent` | Right side content |
| `base` | `Option<HunkContent>` | Base content (if 3-way) |
| `context` | `HunkContext` | Surrounding context |
| `state` | `HunkState` | Resolution state |

---

## HunkState

The state of a single hunk.

| State | Description |
|-------|-------------|
| `Unresolved` | No resolution chosen (default after parsing) |
| `Proposed(Vec<Resolution>)` | Candidate resolutions available (from AST/AI) |
| `Resolved(Resolution)` | Resolution selected |
| `Invalid` | Resolution rejected by validation |

### State Transitions

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

## HunkContent

| Field | Type | Description |
|-------|------|-------------|
| `text` | `String` | The conflicting text |

Future extensions may include:
- Tokenized representations
- AST nodes
- Semantic metadata

---

## HunkContext

Context surrounding the conflict.

| Field | Type | Description |
|-------|------|-------------|
| `before` | `Vec<String>` | Lines before the conflict |
| `after` | `Vec<String>` | Lines after the conflict |
| `start_line_left` | `usize` | Starting line in left version |
| `start_line_right` | `usize` | Starting line in right version |

Used for:
- Display
- Validation
- Structural merging

---

## Resolution

An explicit decision applied to a hunk.

| Field | Type | Description |
|-------|------|-------------|
| `strategy` | `ResolutionStrategy` | How the resolution was chosen |
| `content` | `String` | The resolved content |
| `metadata` | `ResolutionMetadata` | Additional metadata |

---

## ResolutionStrategy

Describes how a resolution was chosen. The strategy is descriptive, not prescriptive—the actual output is always stored explicitly in `content`.

| Variant | Description |
|---------|-------------|
| `AcceptLeft` | Use left content verbatim |
| `AcceptRight` | Use right content verbatim |
| `AcceptBoth(options)` | Combine left and right |
| `Manual` | User-provided content |
| `AstMerged { language }` | Language-specific AST merge |
| `AiSuggested { provider }` | AI-generated suggestion |

---

## AcceptBothOptions

Options for the AcceptBoth strategy.

| Field | Type | Description |
|-------|------|-------------|
| `order` | `BothOrder` | `LeftThenRight` or `RightThenLeft` |
| `deduplicate` | `bool` | Remove duplicate lines |
| `trim_whitespace` | `bool` | Normalize whitespace |

---

## ResolutionMetadata

| Field | Type | Description |
|-------|------|-------------|
| `timestamp` | `DateTime` | When the resolution was made |
| `source` | `Source` | `User`, `Ai`, or `Ast` |
| `notes` | `Option<String>` | Optional notes |

---

## MergeResult

Final output of a merge session.

| Field | Type | Description |
|-------|------|-------------|
| `content` | `String` | The merged file content |
| `unresolved_hunks` | `Vec<HunkId>` | Any hunks that remain unresolved |
| `warnings` | `Vec<MergeWarning>` | Warnings generated |
| `summary` | `MergeSummary` | Statistics |

---

## MergeSummary

| Field | Type | Description |
|-------|------|-------------|
| `total_hunks` | `usize` | Total number of conflict hunks |
| `resolved_hunks` | `usize` | Number resolved |
| `strategies_used` | `Map<Strategy, usize>` | Count per strategy |

---

## Design Guarantees

- All IDs are deterministic
- All decisions are explicit
- No domain object depends on UI, filesystem, or Git
- Resolution ≠ output text (allows replay and strategy changes)
