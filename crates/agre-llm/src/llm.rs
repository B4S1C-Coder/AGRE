use agre_core::Message;
use reqwest::Client;
use thiserror::Error;
use crate::chat::{ChatCompletionRequest, ChatCompletionResponse};

#[derive(Debug)]
pub struct LlmClient {
  client: Client,
  endpoint: String,
  model: String,
}

#[derive(Debug, Error)]
pub enum LlmError {
  #[error("failed to communicate with LLM backend: {0}")]
  Network(#[source] reqwest::Error),

  #[error("LLM backend return HTTP {status}:{body}")]
  HttpStatus {
    status: reqwest::StatusCode,
    body: String,
  },

  #[error("LLM backend returned invalid JSON: {0}")]
  InvalidJson(#[source] serde_json::Error),

  #[error("LLM backend returned an unexpected response: {0}")]
  UnexpectedResponse(String),
}

impl LlmClient {
  pub fn new(base_url: impl Into<String>, model: impl Into<String>) -> Self {
    let base_url = base_url.into();

    Self {
      client: Client::new(),
      endpoint: format!(
        "{}/v1/chat/completions",
        base_url.trim_end_matches('/')
      ),
      model: model.into(),
    }
  }

  pub async fn chat(&self, messages: Vec<Message>) -> Result<String, LlmError> {
    let response = self
      .client
      .post(&self.endpoint)
      .json(&ChatCompletionRequest {
        model: &self.model,
        messages: &messages
      })
      .send()
      .await
      .map_err(LlmError::Network)?;

    let status = response.status();

    if !status.is_success() {
      let body = response.text().await.map_err(LlmError::Network)?;

      return Err(LlmError::HttpStatus { status, body });
    }

    let body = response.text().await.map_err(LlmError::Network)?;

    let completion: ChatCompletionResponse = serde_json::from_str(&body).map_err(LlmError::InvalidJson)?;

    let choice = completion
      .choices
      .into_iter()
      .next()
      .ok_or_else(|| {
        LlmError::UnexpectedResponse(
          "response contained no choices".to_string()
        )
      })?;
    
    choice
      .message
      .content
      .ok_or_else(|| {
        LlmError::UnexpectedResponse(
          "message contained no content".to_string()
        )
      })
  }
}