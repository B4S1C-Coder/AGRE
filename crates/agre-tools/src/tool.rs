use async_trait::async_trait;
use serde_json::Value;

use crate::ToolError;

/// Each tool must implement this trait in order to be recognised by the
/// runtime.
/// ### Why Send + Sync ?
/// This trait is Send + Sync because the runtime is async and registry
/// stores trait objects as `Box<dyn Tool>`, the tool objects therefore
/// cross async task/thread boundaries. This is why we need Send + Sync.
#[async_trait]
pub trait Tool: Send + Sync {
  fn name(&self) -> &str;

  fn schema(&self) -> Value;

  async fn call(&self, args: Value) -> Result<Value, ToolError>;
}

// Send -> Ownership of a value can be safely transferred to another thread.
// i.e. tyoe can be moved to another thread.

// Sync -> It is safe to share immutable references between multiple threads
// concurrently.
