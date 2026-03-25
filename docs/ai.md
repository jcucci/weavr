# AI Integration

AI-assisted merge resolution is opt-in and never auto-applies. All suggestions require explicit user confirmation.

## Feature flags

AI support is compiled in via feature flags:

| Flag | Provider |
|------|----------|
| `ai-claude` | Anthropic Claude |
| `ai-openai` | OpenAI |
| `ai-local` | Local models (Ollama) |
| `ai-all` | All providers |

```bash
cargo install weavr-cli --features ai-claude
```

The base `ai` flag is automatically enabled by any provider flag.

## Configuration

Add an `[ai]` section to your config file:

```toml
[ai]
enabled = true
provider = "claude"
timeout = 30
min_confidence = 0.7
auto_suggest = false
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | bool | — | Enable AI suggestions |
| `provider` | string | — | `claude`, `openai`, or `local` |
| `timeout` | int | — | Request timeout in seconds |
| `min_confidence` | float | — | Minimum confidence threshold (0.0–1.0) |
| `auto_suggest` | bool | — | Automatically suggest when focusing an unresolved hunk |

## Provider setup

### Claude

```toml
[ai]
provider = "claude"
api_key_env = "ANTHROPIC_API_KEY"
model = "claude-sonnet-4-20250514"
max_tokens = 4096
```

Set the API key in your environment:

```bash
export ANTHROPIC_API_KEY="sk-ant-..."
```

### OpenAI

```toml
[ai]
provider = "openai"
api_key_env = "OPENAI_API_KEY"
model = "gpt-4o"
max_tokens = 4096
```

### Local (Ollama)

```toml
[ai]
provider = "local"
endpoint = "http://localhost:11434"
model = "codellama"
```

No API key required. Ensure Ollama is running with the specified model pulled.

## TUI workflow

| Key | Action |
|-----|--------|
| `s` | Suggest resolution for current hunk |
| `S` | Suggest resolution for all unresolved hunks |
| `?` | Explain the conflict (when suggestion is shown) |
| `Enter` | Accept the suggestion |
| `Esc` | Dismiss the suggestion |

Suggestions appear in the result pane with a confidence score. You can review, accept, or dismiss them individually.

## Headless workflow

Use `--strategy ai` in headless mode:

```bash
weavr --headless --strategy ai --fallback-strategy left
```

When AI declines a suggestion (confidence below threshold) or errors, the `--fallback-strategy` determines what happens. Without a fallback, the hunk remains unresolved.

## Philosophy

- **Opt-in**: AI features are disabled unless explicitly enabled in config
- **Never auto-apply**: Suggestions always require user confirmation (TUI) or explicit `--strategy ai` (headless)
- **Confidence scoring**: Suggestions include a confidence score; low-confidence results can be filtered via `min_confidence`
- **Dismissable**: Every suggestion can be dismissed with `Esc`
- **Transparent**: The AI sees the same conflict context you do — left, right, and base content
