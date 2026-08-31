use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolDefinition {
  #[serde(rename = "type")]
  pub tool_type: ToolType,
  pub function: FunctionDefinition,
}

impl ToolDefinition {
  pub fn function(
    name: impl Into<String>,
    description: impl Into<String>,
    parameters: ToolSchema,
  ) -> Self {
    Self {
      tool_type: ToolType::Function,
      function: FunctionDefinition {
        name: name.into(),
        description: description.into(),
        parameters,
      },
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ToolType {
  Function,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FunctionDefinition {
  pub name: String,
  pub description: String,
  pub parameters: ToolSchema,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolSchema {
  #[serde(rename = "type")]
  pub schema_type: SchemaType,

  pub properties: BTreeMap<String, ParameterSchema>,

  pub required: Vec<String>,

  #[serde(rename = "additionalProperties")]
  pub additional_properties: bool,
}

impl ToolSchema {
  pub fn object(properties: Vec<(&str, ParameterSchema)>, required: &[&str]) -> Self {
    let properties = properties
      .into_iter()
      .map(|(name, schema)| (name.to_string(), schema))
      .collect();

    Self {
      schema_type: SchemaType::Object,
      properties,
      required: required.iter().map(|value| (*value).to_string()).collect(),
      additional_properties: false,
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ParameterSchema {
  #[serde(rename = "type")]
  pub schema_type: SchemaType,

  pub description: String,

  #[serde(rename = "enum", skip_serializing_if = "Option::is_none")]
  pub enum_values: Option<Vec<String>>,
}

impl ParameterSchema {
  pub fn string(description: impl Into<String>) -> Self {
    Self {
      schema_type: SchemaType::String,
      description: description.into(),
      enum_values: None,
    }
  }

  pub fn string_enum(description: impl Into<String>, values: &[&str]) -> Self {
    Self {
      schema_type: SchemaType::String,
      description: description.into(),
      enum_values: Some(values.iter().map(|value| (*value).to_string()).collect()),
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SchemaType {
  String,
  Number,
  Boolean,
  Object,
  Array,
}
