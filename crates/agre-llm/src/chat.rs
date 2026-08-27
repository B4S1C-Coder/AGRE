use agre_core::Message;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub(crate) struct ChatCompletionRequest<'a> {
  pub(crate) model: &'a str,
  pub(crate) messages: &'a [Message],
}

#[derive(Debug, Deserialize)]
pub(crate) struct ChatCompletionResponse {
  pub(crate) choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AssistantMessage {
  pub(crate) content: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Choice {
  pub(crate) message: AssistantMessage,
}