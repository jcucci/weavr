# AST Merge

AST merge performs language-aware structural merging, resolving conflicts at the syntax tree level rather than line-by-line. When AST merge can't handle a conflict (unsupported syntax, ambiguous structure), it falls back to text-based resolution automatically.

## Feature flags

| Flag | Language | Parser |
|------|----------|--------|
| `ast-rust` | Rust | syn |
| `ast-csharp` | C# | tree-sitter |
| `ast-typescript` | TypeScript | tree-sitter |
| `ast-go` | Go | tree-sitter |
| `ast-all` | All of the above | — |

```bash
cargo install weavr-cli --features ast-rust,ast-typescript
```

The base `ast` flag is automatically enabled by any language flag.

## How it works

1. Both sides of a conflict are parsed into syntax trees
2. Structural diffing identifies changes at the declaration level (imports, functions, types)
3. Non-overlapping changes are merged automatically with a confidence score
4. Overlapping or ambiguous changes fall back to text-based merging

### When fallback occurs

- Unsupported file type (no matching language feature compiled)
- Parse errors in either side of the conflict
- Overlapping structural changes (e.g., both sides modify the same function body)
- Confidence score below threshold

## TUI workflow

| Key | Action |
|-----|--------|
| `a` | AST merge current hunk |
| `A` | AST merge all unresolved hunks |
| `Enter` | Accept AST suggestion |
| `Esc` | Dismiss suggestion |

AST suggestions appear in the result pane. Review the merged result before accepting.

## Headless workflow

```bash
weavr --headless --strategy ast
```

In headless mode, AST merge is attempted for each hunk. Hunks that can't be structurally merged fall back to the configured fallback strategy (defaults to `left`).

## Supported merge operations

- **Import deduplication**: Merges import/use statements without duplicates
- **Declaration reordering**: Handles cases where both sides add declarations in different positions
- **Non-overlapping edits**: Merges changes to different functions/types in the same file
