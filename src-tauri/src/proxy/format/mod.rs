//! HTTP request/response format conversion between LLM API protocols.

pub mod gemini_schema;
pub mod gemini_shadow;
pub mod streaming;
pub mod streaming_codex_chat;
pub mod streaming_gemini;
pub mod streaming_responses;
pub mod transform;
pub mod transform_codex_chat;
pub mod transform_gemini;
pub mod transform_responses;

use bytes::Bytes;
use futures::stream::{Stream, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum ApiFormat {
    #[default]
    Passthrough,
    AnthropicToOpenaiChat,
    AnthropicToOpenaiResponses,
    AnthropicToGemini,
    ResponsesToOpenaiChat,
}

impl ApiFormat {
    pub fn needs_conversion(self) -> bool {
        !matches!(self, ApiFormat::Passthrough)
    }
}

pub fn transform_request(format: ApiFormat, body: &[u8]) -> Result<Vec<u8>, String> {
    if !format.needs_conversion() || body.is_empty() {
        return Ok(body.to_vec());
    }
    let json: Value = serde_json::from_slice(body)
        .map_err(|e| format!("request body must be JSON for format conversion: {e}"))?;
    let converted = match format {
        ApiFormat::Passthrough => json,
        ApiFormat::AnthropicToOpenaiChat => {
            transform::anthropic_to_openai(json).map_err(|e| e.to_string())?
        }
        ApiFormat::AnthropicToOpenaiResponses => {
            transform_responses::anthropic_to_responses(json, None, false, false)
                .map_err(|e| e.to_string())?
        }
        ApiFormat::AnthropicToGemini => {
            transform_gemini::anthropic_to_gemini(json).map_err(|e| e.to_string())?
        }
        ApiFormat::ResponsesToOpenaiChat => {
            transform_codex_chat::responses_to_chat_completions(json).map_err(|e| e.to_string())?
        }
    };
    serde_json::to_vec(&converted).map_err(|e| e.to_string())
}

pub fn transform_response_json(format: ApiFormat, body: &[u8]) -> Result<Vec<u8>, String> {
    if !format.needs_conversion() || body.is_empty() {
        return Ok(body.to_vec());
    }
    let json: Value = serde_json::from_slice(body)
        .map_err(|e| format!("response body must be JSON for format conversion: {e}"))?;
    let converted = match format {
        ApiFormat::Passthrough => json,
        ApiFormat::AnthropicToOpenaiChat => {
            transform::openai_to_anthropic(json).map_err(|e| e.to_string())?
        }
        ApiFormat::AnthropicToOpenaiResponses => {
            transform_responses::responses_to_anthropic(json).map_err(|e| e.to_string())?
        }
        ApiFormat::AnthropicToGemini => {
            transform_gemini::gemini_to_anthropic(json).map_err(|e| e.to_string())?
        }
        ApiFormat::ResponsesToOpenaiChat => {
            transform_codex_chat::chat_completion_to_response(json).map_err(|e| e.to_string())?
        }
    };
    serde_json::to_vec(&converted).map_err(|e| e.to_string())
}

pub fn wrap_response_stream<E>(
    format: ApiFormat,
    stream: impl Stream<Item = Result<Bytes, E>> + Send + 'static,
) -> std::pin::Pin<Box<dyn Stream<Item = Result<Bytes, std::io::Error>> + Send>>
where
    E: std::error::Error + Send + 'static,
{
    match format {
        ApiFormat::Passthrough => Box::pin(stream.map(|r| {
            r.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
        })),
        ApiFormat::AnthropicToOpenaiChat => Box::pin(streaming::create_anthropic_sse_stream(stream)),
        ApiFormat::AnthropicToOpenaiResponses => Box::pin(
            streaming_responses::create_anthropic_sse_stream_from_responses(stream),
        ),
        ApiFormat::AnthropicToGemini => Box::pin(
            streaming_gemini::create_anthropic_sse_stream_from_gemini(
                stream, None, None, None, None,
            ),
        ),
        ApiFormat::ResponsesToOpenaiChat => Box::pin(
            streaming_codex_chat::create_responses_sse_stream_from_chat(stream),
        ),
    }
}
