# CLAUDE.md

This document defines the expectations, constraints, and guiding principles for human and AI contributors working on meldr.

## Project Intent

meldr is a terminal-first merge conflict resolver built on a structured merge engine.
Conflicts are treated as explicit domain objects, not raw text.

The core philosophy is:
- Explicit over implicit
- Assistive automation over opaque automation
- Deterministic, replayable merges

## Golden Rules

1. **meldr-core must remain pure**
   - No filesystem access
   - No Git invocation
   - No UI dependencies
   - No network access

2. **No hidden decisions**
   - All merge resolutions must be explicit
   - AI suggestions must never auto-apply

3. **Determinism is mandatory**
   - Given the same inputs and decisions, output must be identical

4. **Graceful fallback**
   - Structured or AST-based merging must always fall back to text-based merging

5. **Terminal-first**
   - Keyboard-driven workflows are the primary UX
   - Mouse support is optional and secondary

## Contribution Guidelines

- Prefer extending the domain model over adding flags or special cases
- Public APIs should be designed before implementation
- Large changes should be discussed in an issue before coding

## AI Agent Guidance

AI agents may:
- Propose merge strategies
- Suggest resolutions
- Generate explanations

AI agents must not:
- Apply changes without explicit user confirmation
- Modify merge results implicitly
- Bypass validation rules

All AI-assisted features must be opt-in and clearly labeled.
