# Terminology

This document defines key terms used throughout the meldr codebase and documentation.

---

## Core Concepts

### Conflict

A region in a file where two versions (left and right) differ incompatibly and cannot be automatically merged by Git.

In Git, conflicts are marked with:
```
<<<<<<< HEAD
left content
=======
right content
>>>>>>> branch-name
```

meldr treats conflicts as **structured data**, not raw text.

### Hunk

A **contiguous** region of conflicting content within a file. A single file may contain multiple hunks.

Each hunk has:
- An identifier (`HunkId`)
- Left content
- Right content
- Optional base content
- Surrounding context
- A resolution state

### Resolution

An **explicit decision** about how to resolve a single hunk. A resolution includes:
- The chosen strategy (how the decision was made)
- The resolved content (what the output will be)
- Metadata (when, by whom, notes)

Resolutions are **inert** until applied—they represent decisions, not output.

### Strategy

The **method** used to choose a resolution. Strategies are descriptive, not prescriptive.

Built-in strategies:
- `AcceptLeft` — use left content
- `AcceptRight` — use right content
- `AcceptBoth` — combine both sides
- `Manual` — user-provided content
- `AstMerged` — language-aware structural merge
- `AiSuggested` — AI-generated suggestion

### Session

A `MergeSession` represents **one merge attempt for one file**. It tracks:
- The original input
- All hunks
- Applied resolutions
- Current state

Sessions progress through a defined lifecycle: Parsed → Active → FullyResolved → Applied → Validated.

---

## File Versions

### Left

The **HEAD** version—your current branch's content. Sometimes called "ours" in Git terminology.

### Right

The **MERGE_HEAD** version—the incoming branch's content. Sometimes called "theirs" in Git terminology.

### Base

The **common ancestor** version—the content before either branch made changes. Used in 3-way merges. Optional (not always available).

---

## States

### MergeState

The state of a `MergeSession`:

| State | Description |
|-------|-------------|
| Uninitialized | Input provided, not parsed |
| Parsed | Markers parsed, hunks created |
| Active | Resolution in progress |
| FullyResolved | All hunks resolved |
| Applied | Output generated |
| Validated | Output verified |
| Completed | Final result returned |

### HunkState

The state of a single hunk:

| State | Description |
|-------|-------------|
| Unresolved | No resolution chosen |
| Proposed | Candidates available |
| Resolved | Resolution selected |
| Invalid | Resolution rejected |

---

## Operations

### Parse

Convert raw conflict markers into structured `ConflictHunk` objects.

### Propose

Generate candidate resolutions for a hunk using configured strategies. Does not modify state.

### Apply (resolution)

Select a specific resolution for a hunk. Modifies session state.

### Apply (session)

Generate the merged output text from all resolutions.

### Validate

Verify that the output contains no conflict markers and is valid.

### Complete

Finalize the session and return an immutable `MergeResult`.

---

## Architectural Terms

### meldr-core

The pure merge logic library. Has no dependencies on IO, Git, or UI.

### meldr-cli

The command-line interface. Handles file operations and orchestration.

### meldr-tui

The terminal user interface. Built on ratatui.

### meldr-git

Git integration layer. Detects conflicts, stages files.

### meldr-ast

Language-aware AST merging. Supports structural merge strategies.

### meldr-ai

AI provider integrations. Generates suggestions (never auto-applies).

---

## UI Terms

### Three-Pane Layout

The default TUI layout:
- **Left pane** — HEAD content
- **Right pane** — MERGE_HEAD content
- **Result pane** — Resolved/preview content

### Hunk Navigation

Moving between conflict hunks using keyboard shortcuts (typically `j`/`k` or `n`/`p`).

### Ghost Text

Dimmed preview text showing a proposed resolution before it's accepted.

---

## Merge Types

### 2-Way Merge

A merge with only left and right versions (no base). Less context available for resolution.

### 3-Way Merge

A merge with left, right, and base versions. Enables smarter resolution by understanding what each side changed from the common ancestor.

---

## Strategy Options

### AcceptBoth Options

When using `AcceptBoth` strategy:

| Option | Description |
|--------|-------------|
| Order | `LeftThenRight` or `RightThenLeft` |
| Deduplicate | Remove identical lines from both sides |
| Trim whitespace | Normalize whitespace before comparison |

---

## Error Types

### ParseError

Failed to parse conflict markers from input.

### ResolutionError

Invalid resolution operation (e.g., hunk not found).

### ValidationError

Output failed validation (markers remain, syntax error).

### ApplyError

Failed to generate output from resolutions.
