//! Local LLM provider implementation (e.g., Ollama).

use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use weavr_core::{
    ConflictHunk, Resolution, ResolutionMetadata, ResolutionSource, ResolutionStrategyKind,
};

use crate::error::AiError;
use crate::request::{AiRequest, AiResponse};
use crate::AiProvider;

/// Ollama chat API response.
#[derive(Deserialize)]
struct OllamaChatResponse {
    message: OllamaMessage,
}

/// Message content in an Ollama chat response.
#[derive(Deserialize)]
struct OllamaMessage {
    content: Option<String>,
}

/// Raw AI response with f32 confidence (as returned by the model).
#[derive(Deserialize)]
struct RawAiResponse {
    suggestion: String,
    confidence: f32,
    explanation: Option<String>,
}

fn default_endpoint() -> String {
    "http://localhost:11434".into()
}

fn default_model() -> String {
    "codellama".into()
}

/// Local LLM provider configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LocalConfig {
    /// Endpoint URL (base URL, e.g., `http://localhost:11434`).
    #[serde(default = "default_endpoint")]
    pub endpoint: String,

    /// Model name.
    #[serde(default = "default_model")]
    pub model: String,
}

impl Default for LocalConfig {
    fn default() -> Self {
        Self {
            endpoint: default_endpoint(),
            model: default_model(),
        }
    }
}

/// Local LLM provider.
pub struct LocalProvider {
    endpoint: String,
    model: String,
    timeout: Duration,
    client: reqwest::Client,
}

impl LocalProvider {
    /// Creates a new local LLM provider from configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP client cannot be built.
    pub fn new(config: &LocalConfig) -> Result<Self, AiError> {
        Self::with_timeout(config, Duration::from_secs(120))
    }

    /// Creates a new local LLM provider with a custom timeout.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP client cannot be built.
    pub fn with_timeout(config: &LocalConfig, timeout: Duration) -> Result<Self, AiError> {
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|e| {
                AiError::ProviderNotAvailable(format!("failed to build HTTP client: {e}"))
            })?;

        Ok(Self {
            endpoint: config.endpoint.clone(),
            model: config.model.clone(),
            timeout,
            client,
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

    /// Parses the Ollama chat response into an `AiResponse`.
    fn parse_response(response_body: &str) -> Result<AiResponse, AiError> {
        let ollama_response: OllamaChatResponse = serde_json::from_str(response_body)
            .map_err(|e| AiError::ParseError(format!("failed to parse Ollama response: {e}")))?;

        let text = ollama_response
            .message
            .content
            .ok_or_else(|| AiError::ParseError("no content in Ollama response".into()))?;

        // Clean up the response text - models sometimes wrap JSON in code fences
        let cleaned = Self::extract_json(&text);

        // Parse the raw JSON (with f32 confidence)
        let raw: RawAiResponse = serde_json::from_str(cleaned).map_err(|e| {
            let truncated: String = text.chars().take(200).collect();
            let suffix = if text.len() > 200 { "..." } else { "" };
            AiError::ParseError(format!(
                "failed to parse AI response JSON: {e}\nRaw text: {truncated}{suffix}"
            ))
        })?;

        // Convert f32 confidence (0.0-1.0) to u8 percentage (0-100)
        // The clamp ensures value is in [0.0, 100.0], so truncation and sign loss are safe.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let confidence = (raw.confidence * 100.0).round().clamp(0.0, 100.0) as u8;

        Ok(AiResponse {
            suggestion: raw.suggestion,
            confidence,
            explanation: raw.explanation,
        })
    }

    /// Sends a request and maps timeout errors to `AiError::Timeout`.
    async fn send_request(
        &self,
        request: reqwest::RequestBuilder,
    ) -> Result<reqwest::Response, AiError> {
        request.send().await.map_err(|e| {
            if e.is_timeout() {
                AiError::Timeout(self.timeout)
            } else {
                AiError::NetworkError(e)
            }
        })
    }

    /// Extracts JSON from text that may be wrapped in code fences.
    fn extract_json(text: &str) -> &str {
        let trimmed = text.trim();

        // Strip ```json or ``` prefix
        let without_prefix = trimmed
            .strip_prefix("```json")
            .or_else(|| trimmed.strip_prefix("```"))
            .map_or(trimmed, str::trim_start);

        // Strip trailing ```
        without_prefix
            .strip_suffix("```")
            .map_or(without_prefix, str::trim_end)
    }
}

#[async_trait]
impl AiProvider for LocalProvider {
    fn name(&self) -> &'static str {
        "local"
    }

    async fn suggest(&self, hunk: &ConflictHunk) -> Result<Option<Resolution>, AiError> {
        let request = AiRequest::from_hunk(hunk, None);
        let prompt = Self::build_merge_prompt(&request);

        let url = format!("{}/api/chat", self.endpoint.trim_end_matches('/'));

        let request = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "model": self.model,
                "messages": [{
                    "role": "user",
                    "content": prompt
                }],
                "stream": false
            }));

        let response = self.send_request(request).await?;
        let status = response.status();
        if !status.is_success() {
            let status_code = status.as_u16();
            let message = response.text().await.unwrap_or_default();

            return Err(AiError::ProviderError {
                provider: "local".into(),
                status: status_code,
                message,
            });
        }

        let body = response.text().await?;
        let ai_response = Self::parse_response(&body)?;

        Ok(Some(Resolution {
            kind: ResolutionStrategyKind::AiSuggested {
                provider: "local".into(),
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

        let url = format!("{}/api/chat", self.endpoint.trim_end_matches('/'));

        let request = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "model": self.model,
                "messages": [{
                    "role": "user",
                    "content": prompt
                }],
                "stream": false
            }));

        let response = self.send_request(request).await?;
        let status = response.status();
        if !status.is_success() {
            let status_code = status.as_u16();
            let message = response.text().await.unwrap_or_default();

            return Err(AiError::ProviderError {
                provider: "local".into(),
                status: status_code,
                message,
            });
        }

        let body = response.text().await?;
        let ollama_response: OllamaChatResponse = serde_json::from_str(&body)
            .map_err(|e| AiError::ParseError(format!("failed to parse Ollama response: {e}")))?;

        Ok(ollama_response.message.content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let config = LocalConfig::default();
        assert_eq!(config.endpoint, "http://localhost:11434");
        assert_eq!(config.model, "codellama");
    }

    #[test]
    fn deserialize_config() {
        let toml = r#"
            endpoint = "http://myhost:8080"
            model = "llama3"
        "#;

        let config: LocalConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.endpoint, "http://myhost:8080");
        assert_eq!(config.model, "llama3");
    }

    #[test]
    fn deserialize_config_with_defaults() {
        let toml = "";
        let config: LocalConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.endpoint, "http://localhost:11434");
        assert_eq!(config.model, "codellama");
    }

    #[test]
    fn parse_ollama_chat_response() {
        let json = r#"{
            "message": {
                "content": "{\"suggestion\": \"resolved code\", \"confidence\": 0.85, \"explanation\": \"Merged both changes\"}"
            }
        }"#;

        let response = LocalProvider::parse_response(json).unwrap();
        assert_eq!(response.suggestion, "resolved code");
        assert_eq!(response.confidence, 85);
        assert_eq!(response.explanation, Some("Merged both changes".into()));
    }

    #[test]
    fn parse_raw_ai_response() {
        let json =
            r#"{"suggestion": "merged code", "confidence": 0.9, "explanation": "Combined both"}"#;
        let raw: RawAiResponse = serde_json::from_str(json).unwrap();
        assert_eq!(raw.suggestion, "merged code");
        assert!((raw.confidence - 0.9).abs() < f32::EPSILON);
        assert_eq!(raw.explanation, Some("Combined both".into()));

        // Test conversion to u8 percentage
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let confidence = (raw.confidence * 100.0).round() as u8;
        assert_eq!(confidence, 90);
    }

    #[test]
    fn extract_json_plain() {
        let text = r#"{"suggestion": "code", "confidence": 0.8}"#;
        assert_eq!(LocalProvider::extract_json(text), text);
    }

    #[test]
    fn extract_json_with_fences() {
        let text = "```json\n{\"suggestion\": \"code\"}\n```";
        assert_eq!(
            LocalProvider::extract_json(text),
            "{\"suggestion\": \"code\"}"
        );
    }

    #[test]
    fn extract_json_with_plain_fences() {
        let text = "```\n{\"suggestion\": \"code\"}\n```";
        assert_eq!(
            LocalProvider::extract_json(text),
            "{\"suggestion\": \"code\"}"
        );
    }
}
