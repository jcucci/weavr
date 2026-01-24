# Resolution Strategies

Resolution strategies describe **how** a resolution was chosen, not how it is applied. The strategy is descriptive—the resolved content is always stored explicitly.

---

## Strategy Trait

All strategies implement a common interface:

```rust
pub trait ResolutionStrategy {
    fn propose(&self, hunk: &ConflictHunk) -> Option<Resolution>;
    fn kind(&self) -> ResolutionStrategyKind;
}
```

The engine orchestrates strategies:

```rust
pub struct MergeEngine {
    pub strategies: Vec<Box<dyn ResolutionStrategy>>,
}
```

---

## ResolutionStrategyKind

This enum describes the **source** of a resolution (imported from domain-model):

```rust
pub enum ResolutionStrategyKind {
    AcceptLeft,
    AcceptRight,
    AcceptBoth(AcceptBothOptions),
    Manual,
    AstMerged { language: Language },
    AiSuggested { provider: String },
}
```

The `ResolutionStrategy` trait generates proposals; `ResolutionStrategyKind` records **how** a resolution was chosen.

---

## Built-in Strategies

### AcceptLeft

Uses the left (HEAD) content verbatim.

```rust
pub struct AcceptLeftStrategy;

impl ResolutionStrategy for AcceptLeftStrategy {
    fn propose(&self, hunk: &ConflictHunk) -> Option<Resolution> { /* ... */ }
    fn kind(&self) -> ResolutionStrategyKind { ResolutionStrategyKind::AcceptLeft }
}
```

**Use case:** Prefer your changes over incoming changes.

### AcceptRight

Uses the right (MERGE_HEAD) content verbatim.

```rust
pub struct AcceptRightStrategy;

impl ResolutionStrategy for AcceptRightStrategy {
    fn propose(&self, hunk: &ConflictHunk) -> Option<Resolution> { /* ... */ }
    fn kind(&self) -> ResolutionStrategyKind { ResolutionStrategyKind::AcceptRight }
}
```

**Use case:** Prefer incoming changes over your changes.

### AcceptBoth

Combines left and right content with configurable options.

```rust
pub struct AcceptBothStrategy {
    pub options: AcceptBothOptions,
}

impl ResolutionStrategy for AcceptBothStrategy {
    fn propose(&self, hunk: &ConflictHunk) -> Option<Resolution> { /* ... */ }
    fn kind(&self) -> ResolutionStrategyKind {
        ResolutionStrategyKind::AcceptBoth(self.options.clone())
    }
}
```

#### AcceptBothOptions

| Option | Type | Description |
|--------|------|-------------|
| `order` | `BothOrder` | `LeftThenRight` or `RightThenLeft` |
| `deduplicate` | `bool` | Remove identical lines that appear in both sides |
| `trim_whitespace` | `bool` | Normalize whitespace before comparison |

**Use cases:**
- Both sides added imports
- Both sides added config entries
- Additive changes that don't conflict semantically

This strategy alone puts weavr ahead of most merge tools.

### Manual

User-provided content that doesn't match any automated strategy.

```rust
pub struct ManualStrategy;

impl ResolutionStrategy for ManualStrategy {
    fn propose(&self, _hunk: &ConflictHunk) -> Option<Resolution> { None }
    fn kind(&self) -> ResolutionStrategyKind { ResolutionStrategyKind::Manual }
}
```

**Use case:** Complex merges requiring human judgment.

---

## AST-Based Strategies

Language-specific structural merging.

```rust
pub struct AstStrategy {
    pub language: Language,
    pub fallback: Box<dyn ResolutionStrategy>,
}

impl ResolutionStrategy for AstStrategy {
    fn propose(&self, hunk: &ConflictHunk) -> Option<Resolution> { /* ... */ }
    fn kind(&self) -> ResolutionStrategyKind {
        ResolutionStrategyKind::AstMerged { language: self.language }
    }
}
```

### Supported Languages

| Language | Status | Capabilities |
|----------|--------|--------------|
| Rust | Planned | Import merging, function-level conflicts |
| C# | Planned | Using statements, method bodies |
| TypeScript | Planned | Import/export merging |
| Go | Planned | Import blocks, struct fields |

### AST Strategy Flow

1. Extract conflicted region
2. Parse both sides as AST fragments
3. Attempt structural merge
4. Format with language formatter
5. Replace result

### Fallback Requirement

**Critical:** AST strategies must **always** fall back to text-based merging if:
- Parsing fails
- Merge is ambiguous
- Language is not supported

```rust
impl ResolutionStrategy for AstStrategy {
    fn propose(&self, hunk: &ConflictHunk) -> Option<Resolution> {
        self.try_ast_merge(hunk)
            .or_else(|| self.fallback.propose(hunk))
    }
}
```

### AST Capabilities by Language

#### Rust
- Merge `use` statements
- Deduplicate imports
- Function-level conflict resolution

#### C#
- Merge `using` statements
- Namespace handling
- Method body conflicts

#### TypeScript
- Import/export merging
- Type declaration conflicts
- Module resolution

#### Go
- Import block merging
- Struct field deduplication
- Interface implementation

---

## AI-Suggested Strategies

AI-assisted resolution is **opt-in** and **never auto-applies**.

```rust
pub struct AiStrategy {
    pub provider: Box<dyn AiProvider>,
    pub provider_name: String,
}

impl ResolutionStrategy for AiStrategy {
    fn propose(&self, hunk: &ConflictHunk) -> Option<Resolution> { /* ... */ }
    fn kind(&self) -> ResolutionStrategyKind {
        ResolutionStrategyKind::AiSuggested { provider: self.provider_name.clone() }
    }
}
```

### Core Principles

1. **Suggestions only** — AI produces proposals, never final decisions
2. **Fully opt-in** — Must be explicitly enabled
3. **Reviewable** — All suggestions can be inspected and edited
4. **Non-blocking** — AI calls are async, UI remains responsive

### AI Capabilities

| Capability | Description |
|------------|-------------|
| Suggest merge | Propose combined resolution |
| Suggest accept both (cleaned) | Deduplicated, ordered combination |
| Explain difference | Natural language explanation |

### Implementation Contract

```rust
pub trait AiProvider {
    async fn suggest(&self, hunk: &ConflictHunk) -> Option<Resolution>;
    async fn explain(&self, hunk: &ConflictHunk) -> Option<String>;
}
```

- Simple JSON in/out contract
- Pluggable providers (Claude, OpenAI, local LLMs)
- Chunk-level operation (per hunk, not whole file)
- Confidence scores optional

### Display in UI

AI suggestions appear as:
- Ghost text in result pane
- One-key accept
- Clear "[AI Suggested]" label

### Example

```
"Both sides added a logging call; the right side includes
structured fields. Suggest keeping both with structured fields."
```

---

## Strategy Resolution Flow

When a user selects a hunk:

1. `weavr-core` requests strategies:
   - Text merge options
   - AST merge (if language supported)
   - AI suggestion (if enabled)
2. Frontend displays available options
3. User chooses one
4. Core applies the resolution

**Key insight:** Frontends never "merge" anything. They just choose.

---

## Generated Resolution Metadata

All "smart" merges include metadata:

```rust
pub struct GeneratedResolution {
    pub source: GenerationSource,
    pub lines: Vec<Line>,
    pub confidence: Option<f32>,
}

pub enum GenerationSource {
    Ast(Language),
    Ai(String), // provider name
}
```

This allows the TUI to display:
- "Suggested by AST (Rust)"
- "Suggested by Claude (confidence: 0.92)"

---

## Strategy Priority

When multiple strategies apply:

1. User's explicit choice (always wins)
2. AST merge (if available and confident)
3. AI suggestion (if enabled)
4. Text-based strategies (AcceptLeft/Right/Both)

The order is configurable per session or via config file.
