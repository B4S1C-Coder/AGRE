use agre_core::{Message, ToolCall, ToolDefinition};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub(crate) struct ChatCompletionRequest<'a> {
  pub(crate) model: &'a str,
  pub(crate) messages: &'a [Message],

  #[serde(skip_serializing_if = "Option::is_none")]
  pub(crate) tools: Option<&'a [ToolDefinition]>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ChatCompletionResponse {
  pub(crate) choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AssistantMessage {
  pub(crate) content: Option<String>,

  #[serde(default)]
  pub(crate) tool_calls: Vec<AssistantToolCall>,
}

impl AssistantMessage {
  pub(crate) fn into_core_message(self) -> Message {
    let tool_calls = self
      .tool_calls
      .into_iter()
      .map(|tool_call| ToolCall {
        id: tool_call.id,
        name: tool_call.function.name,
        arguments: tool_call.function.arguments,
      })
      .collect();

    Message {
      role: agre_core::Role::Assistant,
      content: self.content.unwrap_or_default(),
      tool_calls,
      tool_call_id: None,
    }
  }
}

#[derive(Debug, Deserialize)]
pub(crate) struct Choice {
  pub(crate) message: AssistantMessage,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AssistantToolCall {
  pub(crate) id: String,

  #[serde(rename = "type")]
  pub(crate) call_type: String,

  pub(crate) function: AssistantFunctionCall,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AssistantFunctionCall {
  pub(crate) name: String,
  pub(crate) arguments: String,
}
