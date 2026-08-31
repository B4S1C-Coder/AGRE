use crate::{Tool, ToolError};
use agre_core::{ParameterSchema, ToolSchema};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::path::PathBuf;

pub struct FileTool {
  working_directory: PathBuf,
}

impl FileTool {
  pub fn new(working_directory: impl Into<PathBuf>) -> Result<Self, ToolError> {
    let working_directory = std::fs::canonicalize(working_directory.into())?;
    Ok(Self { working_directory })
  }

  fn resolve_path(&self, path: &str) -> Result<PathBuf, ToolError> {
    let candidate = self.working_directory.join(path);

    let resolved = if candidate.exists() {
      std::fs::canonicalize(candidate)?
    } else {
      let parent = candidate
        .parent()
        .ok_or(ToolError::PathOutsideWorkingDirectory)?;

      let parent = std::fs::canonicalize(parent)?;

      parent.join(
        candidate
          .file_name()
          .ok_or(ToolError::PathOutsideWorkingDirectory)?,
      )
    };

    if !resolved.starts_with(&self.working_directory) {
      return Err(ToolError::PathOutsideWorkingDirectory);
    }

    Ok(resolved)
  }
}

#[async_trait]
impl Tool for FileTool {
  fn name(&self) -> &str {
    "file"
  }

  fn description(&self) -> &str {
    "Read or write a text file inside the configured working directory."
  }

  fn schema(&self) -> ToolSchema {
    ToolSchema::object(
      vec![
        (
          "operation",
          ParameterSchema::string_enum("Whether to read or write the file.", &["read", "write"]),
        ),
        (
          "path",
          ParameterSchema::string("Path relative to the working directory."),
        ),
        (
          "content",
          ParameterSchema::string("Text to write. Required when operation is write."),
        ),
      ],
      &["operation", "path"],
    )
  }

  async fn call(&self, args: Value) -> Result<Value, ToolError> {
    let operation = args
      .get("operation")
      .and_then(Value::as_str)
      .ok_or_else(|| ToolError::InvalidArguments("expected string field 'operation'".into()))?;

    let path = args
      .get("path")
      .and_then(Value::as_str)
      .ok_or_else(|| ToolError::InvalidArguments("expected string field 'path'".into()))?;

    let resolved = self.resolve_path(path)?;

    match operation {
      "read" => {
        let content = std::fs::read_to_string(&resolved)?;

        Ok(json!({
          "content": content
        }))
      }

      "write" => {
        let content = args.get("content").and_then(Value::as_str).ok_or_else(|| {
          ToolError::InvalidArguments("field 'content' is required for 'write'".into())
        })?;

        std::fs::write(&resolved, content)?;

        Ok(json!({
          "written": true,
          "path": path
        }))
      }

      other => Err(ToolError::InvalidArguments(format!(
        "unsupported operation '{other}', expected 'read' or 'write'"
      ))),
    }
  }
}
