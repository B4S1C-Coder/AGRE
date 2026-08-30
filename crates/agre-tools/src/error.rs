use thiserror::Error;

#[derive(Debug, Error)]
pub enum ToolError {
  #[error("invalid arguments: {0}")]
  InvalidArguments(String),

  #[error("tool I/O error: {0}")]
  Io(#[from] std::io::Error),

  #[error("HTTP request failed: {0}")]
  Http(#[from] reqwest::Error),

  #[error("path is outside the tool working directory")]
  PathOutsideWorkingDirectory,

  #[error("file does not exist: {0}")]
  FileNotFound(String),

  #[error("tool execution failed: {0}")]
  Execution(String),
}