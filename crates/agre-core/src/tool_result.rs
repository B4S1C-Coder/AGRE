use crate::{Message, Role};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub struct ToolResult {
  pub call_id: String,
  pub output: Result<Value, String>,
}

impl ToolResult {
  pub fn into_message(self) -> Message {
    let content = match self.output {
      Ok(value) => value.to_string(),
      Err(error) => serde_json::json!({
        "error": error
      }).to_string(),
    };

    Message {
      role: Role::Tool,
      content,
      tool_calls: Vec::new(),
      tool_call_id: Some(self.call_id),
    }
  }
}
