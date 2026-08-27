use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolCall {
  pub id: String,
  pub name: String,
  pub arguments: serde_json::Value,
}