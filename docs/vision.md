
# Vision

weavr exists to make merge conflicts safer, more explicit, and less frustrating for
terminal-first developers.

Most merge tools treat conflicts as plain text problems. weavr treats them as structured
decisions made by a human, optionally assisted by automation.

## Core Principles

### 1. Conflicts are structured data
A merge conflict is not just text; it has:
- A left side
- A right side
- Optional base content
- Context
- A decision history

weavr models these explicitly.

### 2. No hidden decisions
Automation may assist, but it must never silently decide.
Every resolution is visible, explicit, and reversible.

### 3. Deterministic outcomes
Given the same inputs and the same decisions, weavr must always produce the same output.

### 4. Terminal-first
weavr prioritizes keyboard-driven, terminal-based workflows.
Editor and UI integrations are built on top of this foundation.

### 5. Progressive enhancement
Text-based merging is always available.
AST-based and AI-assisted merging enhance—but never replace—it.
