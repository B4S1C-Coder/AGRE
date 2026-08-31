use agre_core::{ParameterSchema, ToolSchema};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::path::PathBuf;

use crate::{Tool, ToolError};

pub struct DeleteFileTool {
  working_directory: PathBuf,
}

impl DeleteFileTool {
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
impl Tool for DeleteFileTool {
  fn name(&self) -> &str {
    "delete_file"
  }

  fn description(&self) -> &str {
    "Delete a file inside the configured working directory."
  }

  fn schema(&self) -> ToolSchema {
    ToolSchema::object(
      vec![(
        "path",
        ParameterSchema::string("Path of the file to delete, relative to the working directory"),
      )],
      &["path"],
    )
  }

  async fn call(&self, args: Value) -> Result<Value, ToolError> {
    let path = args
      .get("path")
      .and_then(Value::as_str)
      .ok_or_else(|| ToolError::InvalidArguments("expected string field 'path'".into()))?;

    let resolved = self.resolve_path(path)?;

    std::fs::remove_file(&resolved)?;

    Ok(json!({
      "deleted": true,
      "path": path
    }))
  }
}
