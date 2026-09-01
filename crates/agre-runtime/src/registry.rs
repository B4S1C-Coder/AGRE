use std::collections::HashMap;
use agre_core::ToolDefinition;
use agre_tools::Tool;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RegistryError {
  #[error("tool name cannot be empty")]
  EmptyToolName,

  #[error("tool '{0}' is already registered")]
  DuplicateTool(String),
}

pub struct ToolRegistry {
  tools: HashMap<String, Box<dyn Tool>>,
}

impl ToolRegistry {
  pub fn new() -> Self {
    Self {
      tools: HashMap::new(),
    }
  }

  pub fn register<T>(&mut self, tool: T) -> Result<(), RegistryError>
  where 
    T: Tool + 'static
  {
    let name = tool.name().to_string();

    if name.is_empty() {
      return Err(RegistryError::EmptyToolName);
    }

    if self.tools.contains_key(&name) {
      return Err(RegistryError::DuplicateTool(name));
    }

    self.tools.insert(name, Box::new(tool));

    Ok(())
  }

  pub fn get(&self, name: &str) -> Option<&dyn Tool> {
    self.tools.get(name).map(Box::as_ref)
  }

  pub fn definitions(&self) -> Vec<ToolDefinition> {
    let mut definitions: Vec<ToolDefinition> = self
      .tools
      .values()
      .map(|tool| {
        ToolDefinition::function(tool.name(), tool.description(), tool.schema())
      })
      .collect();

    definitions.sort_by(|left, right| {
      left.function.name.cmp(&right.function.name)
    });

    definitions
  }
}

impl Default for ToolRegistry {
  fn default() -> Self {
      Self::new()
  }
}