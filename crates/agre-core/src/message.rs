use crate::Role;
use crate::ToolCall;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Message {
  pub role: Role,
  pub content: String,

  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub tool_calls: Vec<ToolCall>,

  #[serde(skip_serializing_if = "Option::is_none")]
  pub tool_call_id: Option<String>,
}
