use agre_core::{ParameterSchema, ToolSchema};
use async_trait::async_trait;
use serde_json::{Value, json};

use crate::{Tool, ToolError};

pub struct HttpFetch {
  client: reqwest::Client,
}

impl HttpFetch {
  pub fn new() -> Self {
    Self {
      client: reqwest::Client::new(),
    }
  }
}

impl Default for HttpFetch {
  fn default() -> Self {
    Self::new()
  }
}

#[async_trait]
impl Tool for HttpFetch {
  fn name(&self) -> &str {
    "http_fetch"
  }

  fn description(&self) -> &str {
    "Fetch the contents of a URL using an HTTP GET request."
  }

  fn schema(&self) -> ToolSchema {
    ToolSchema::object(
      vec![(
        "url",
        ParameterSchema::string("The URL to fetch using HTTP GET."),
      )],
      &["url"],
    )
  }

  async fn call(&self, args: Value) -> Result<Value, ToolError> {
    let url = args
      .get("url")
      .and_then(Value::as_str)
      .ok_or_else(|| ToolError::InvalidArguments("expected string field 'url'".into()))?;

    let response = self.client.get(url).send().await?.error_for_status()?;

    let status = response.status().as_u16();
    let body = response.text().await?;

    Ok(json!({
      "status": status,
      "body": body
    }))
  }
}
