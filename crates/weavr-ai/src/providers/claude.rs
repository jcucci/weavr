//! Claude AI provider implementation.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use weavr_core::{ConflictHunk, Resolution, ResolutionMetadata, ResolutionSource, ResolutionStrategyKind};

use crate::error::AiError;
use crate::request::{AiRequest, AiResponse};
use crate::AiProvider;

/// Claude API response structure.
#[derive(Deserialize)]
struct ClaudeApiResponse {
    content: Vec<ContentBlock>,
}

/// Content block in Claude API response.
#[derive(Deserialize)]
struct ContentBlock {
    text: Option<String>,
}

/// Claude provider configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ClaudeConfig {
    /// Environment variable containing the API key.
    #[serde(default = "default_api_key_env")]
    pub api_key_env: String,

    /// Model to use.
    #[serde(default = "default_model")]
    pub model: String,

    /// Maximum tokens in response.
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
}

impl Default for ClaudeConfig {
    fn default() -> Self {
        Self {
            api_key_env: default_api_key_env(),
            model: default_model(),
            max_tokens: default_max_tokens(),
        }
    }
}

fn default_api_key_env() -> String {
    "ANTHROPIC_API_KEY".into()
}

fn default_model() -> String {
    "claude-sonnet-4-20250514".into()
}

fn default_max_tokens() -> u32 {
    4096
}

/// Claude AI provider.
pub struct ClaudeProvider {
    api_key: String,
    model: String,
    max_tokens: u32,
    client: reqwest::Client,
}

impl ClaudeProvider {
    /// Creates a new Claude provider from configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if the API key environment variable is not set.
    pub fn new(config: &ClaudeConfig) -> Result<Self, AiError> {
        let api_key = std::env::var(&config.api_key_env).map_err(|_| {
            AiError::ApiKeyError(format!(
                "environment variable {} not set",
                config.api_key_env
            ))
        })?;

        Ok(Self {
            api_key,
            model: config.model.clone(),
            max_tokens: config.max_tokens,
            client: reqwest::Client::new(),
        })
    }

    /// Builds a prompt for merge conflict resolution.
    fn build_merge_prompt(request: &AiRequest) -> String {
        let base_section = request
            .base
            .as_ref()
            .map(|b| format!("\nBase (common ancestor):\n```\n{b}\n```\n"))
            .unwrap_or_default();

        let language_hint = request
            .context
            .language
            .as_ref()
            .map(|l| format!("\nLanguage: {l}"))
            .unwrap_or_default();

        format!(
            r#"You are a merge conflict resolver. Given two versions of code that conflict, suggest a merged resolution.

Left (ours/HEAD):
```
{}
```

Right (theirs/incoming):
```
{}
```
{base_section}
Context before conflict: {:?}
Context after conflict: {:?}
{language_hint}

Respond with ONLY valid JSON (no markdown, no explanation outside JSON):
{{
  "suggestion": "the merged content exactly as it should appear",
  "confidence": 0.85,
  "explanation": "brief explanation of how you merged the changes"
}}

Important:
- The "suggestion" field must contain the exact merged content
- Confidence should be 0.0-1.0 based on how certain you are
- Preserve original formatting, indentation, and line endings"#,
            request.left, request.right, request.context.before, request.context.after
        )
    }

    /// Builds a prompt for explaining a conflict.
    fn build_explain_prompt(request: &AiRequest) -> String {
        let base_section = request
            .base
            .as_ref()
            .map(|b| format!("\nBase (common ancestor):\n```\n{b}\n```\n"))
            .unwrap_or_default();

        format!(
            r"You are a merge conflict analyzer. Explain the differences between these two versions of code.

Left (ours/HEAD):
```
{}
```

Right (theirs/incoming):
```
{}
```
{base_section}

Provide a clear, concise explanation of:
1. What changed on the left side
2. What changed on the right side
3. Why they conflict
4. Suggestions for resolution

Keep the explanation brief and technical.",
            request.left, request.right
        )
    }

    /// Parses the Claude API response into an `AiResponse`.
    fn parse_response(response_body: &str) -> Result<AiResponse, AiError> {
        let claude_response: ClaudeApiResponse =
            serde_json::from_str(response_body).map_err(|e| {
                AiError::ParseError(format!("failed to parse Claude response: {e}"))
            })?;

        let text = claude_response
            .content
            .into_iter()
            .find_map(|c| c.text)
            .ok_or_else(|| AiError::ParseError("no text in Claude response".into()))?;

        // Parse the JSON from the text content
        serde_json::from_str(&text)
            .map_err(|e| AiError::ParseError(format!("failed to parse AI response JSON: {e}")))
    }
}

#[async_trait]
impl AiProvider for ClaudeProvider {
    fn name(&self) -> &'static str {
        "claude"
    }

    async fn suggest(&self, hunk: &ConflictHunk) -> Result<Option<Resolution>, AiError> {
        let request = AiRequest::from_hunk(hunk, None);
        let prompt = Self::build_merge_prompt(&request);

        let response = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&serde_json::json!({
                "model": self.model,
                "max_tokens": self.max_tokens,
                "messages": [{
                    "role": "user",
                    "content": prompt
                }]
            }))
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let status_code = status.as_u16();
            let message = response.text().await.unwrap_or_default();

            // Check for rate limiting
            if status_code == 429 {
                return Err(AiError::RateLimited {
                    provider: "claude".into(),
                    retry_after_secs: None,
                });
            }

            return Err(AiError::ProviderError {
                provider: "claude".into(),
                status: status_code,
                message,
            });
        }

        let body = response.text().await?;
        let ai_response = Self::parse_response(&body)?;

        Ok(Some(Resolution {
            kind: ResolutionStrategyKind::AiSuggested {
                provider: "claude".into(),
            },
            content: ai_response.suggestion,
            metadata: ResolutionMetadata {
                source: ResolutionSource::Ai,
                notes: ai_response.explanation,
                confidence: Some(ai_response.confidence),
            },
        }))
    }

    async fn explain(&self, hunk: &ConflictHunk) -> Result<Option<String>, AiError> {
        let request = AiRequest::from_hunk(hunk, None);
        let prompt = Self::build_explain_prompt(&request);

        let response = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&serde_json::json!({
                "model": self.model,
                "max_tokens": self.max_tokens,
                "messages": [{
                    "role": "user",
                    "content": prompt
                }]
            }))
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let status_code = status.as_u16();
            let message = response.text().await.unwrap_or_default();
            return Err(AiError::ProviderError {
                provider: "claude".into(),
                status: status_code,
                message,
            });
        }

        let body = response.text().await?;
        let claude_response: ClaudeApiResponse = serde_json::from_str(&body)
            .map_err(|e| AiError::ParseError(format!("failed to parse Claude response: {e}")))?;

        let text = claude_response.content.into_iter().find_map(|c| c.text);

        Ok(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let config = ClaudeConfig::default();
        assert_eq!(config.api_key_env, "ANTHROPIC_API_KEY");
        assert_eq!(config.model, "claude-sonnet-4-20250514");
        assert_eq!(config.max_tokens, 4096);
    }

    #[test]
    fn deserialize_config() {
        let toml = r#"
            api_key_env = "MY_CLAUDE_KEY"
            model = "claude-opus-4-20250514"
            max_tokens = 8192
        "#;

        let config: ClaudeConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.api_key_env, "MY_CLAUDE_KEY");
        assert_eq!(config.model, "claude-opus-4-20250514");
        assert_eq!(config.max_tokens, 8192);
    }

    #[test]
    fn parse_ai_response() {
        let json = r#"{"suggestion": "merged code", "confidence": 0.9, "explanation": "Combined both"}"#;
        let response: AiResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.suggestion, "merged code");
        assert!((response.confidence - 0.9).abs() < f32::EPSILON);
        assert_eq!(response.explanation, Some("Combined both".into()));
    }
}
