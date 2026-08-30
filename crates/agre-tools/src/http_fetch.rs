use async_trait::async_trait;
use serde_json::{json, Value};

use crate::{Tool, ToolError};

pub struct HttpFetch {
  client: reqwest::Client,
}

impl HttpFetch {
  pub fn new() -> Self {
    Self { client: reqwest::Client::new() }
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

  fn schema(&self) -> Value {
    json!({
      "type": "object",
      "properties": {
        "url": {
          "type": "string",
          "description": "URL to fetch using HTTP GET"
        }
      },
      "required": ["url"],
      "additionalProperties": false
    })
  }

  async fn call(&self, args: Value) -> Result<Value, ToolError> {
    let url = args
      .get("url")
      .and_then(Value::as_str)
      .ok_or_else(|| {
        ToolError::InvalidArguments("expected string field 'url'".into())
      })?;
    
    let response = self.client
      .get(url)
      .send()
      .await?
      .error_for_status()?;

    let status = response.status().as_u16();
    let body  = response.text().await?;

    Ok(json!({
      "status": status,
      "body": body
    }))
  }
}