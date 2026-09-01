use agre_core::{Message, Role, ToolResult};
use agre_llm::{LlmClient, LlmError};
use serde_json::Value;
use thiserror::Error;

use crate::ToolRegistry;

pub struct AgentRuntime {
  max_iterations: usize,
}

#[derive(Debug, Error)]
pub enum RuntimeError {
  #[error("LLM request failed: {0}")]
  Llm(#[from] LlmError),

  #[error("maximum agent iterations reached")]
  MaxIterations,
}

impl AgentRuntime {
  pub fn new(max_iterations: usize) -> Self {
    Self { max_iterations }
  }

  pub async fn run(
    &self,
    client: &LlmClient,
    system_prompt: &str,
    user_task: &str,
    registry: &ToolRegistry,
  ) -> Result<String, RuntimeError> {
    let mut messages = vec![
      Message {
        role: Role::System,
        content: system_prompt.to_string(),
        tool_calls: Vec::new(),
        tool_call_id: None,
      },
      Message {
        role: Role::User,
        content: user_task.to_string(),
        tool_calls: Vec::new(),
        tool_call_id: None,
      },
    ];

    let tool_definitions = registry.definitions();

    for _ in 0..self.max_iterations {
      let assistant_message = client.chat_with_tools(&messages, &tool_definitions).await?;

      let tool_calls = assistant_message.tool_calls.clone();

      messages.push(assistant_message.clone());

      if tool_calls.is_empty() {
        return Ok(assistant_message.content);
      }

      for tool_call in tool_calls {
        let tool_result = match serde_json::from_str::<Value>(&tool_call.arguments) {
          Ok(arguments) => match registry.get(&tool_call.name) {
            Some(tool) => match tool.call(arguments).await {
              Ok(output) => ToolResult {
                call_id: tool_call.id.clone(),
                output: Ok(output),
              },

              Err(error) => ToolResult {
                call_id: tool_call.id.clone(),
                output: Err(error.to_string()),
              },
            },

            None => ToolResult {
              call_id: tool_call.id.clone(),
              output: Err(format!("unkown tool '{}'", tool_call.name)),
            },
          },

          Err(error) => ToolResult {
            call_id: tool_call.id.clone(),
            output: Err(format!(
              "malformed JSON arguments for tool '{}': {}. Please retry the tool call with valid JSON arguments.",
              tool_call.name, error
            )),
          },
        };

        messages.push(tool_result.into_message());
      }
    }

    Err(RuntimeError::MaxIterations)
  }
}
