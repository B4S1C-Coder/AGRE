use std::path::PathBuf;

use crate::{Tool, ToolError};

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
      let parent  = candidate
        .parent()
        .ok_or(ToolError::PathOutsideWorkingDirectory)?;

      let parent = std::fs::canonicalize(parent)?;

      parent.join(
        candidate.file_name().ok_or(ToolError::PathOutsideWorkingDirectory)?
      )
    };

    if !resolved.starts_with(&self.working_directory) {
      return Err(ToolError::PathOutsideWorkingDirectory);
    }

    Ok(resolved)
  }
}
