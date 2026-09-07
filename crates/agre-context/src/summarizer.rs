use agre_core::{Message, Role};
use agre_llm::{LlmClient, LlmError};
use async_trait::async_trait;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SummarizeError {
  #[error("summarizer LLM request failed: {0}")]
  Llm(#[from] LlmError),

  #[error("summarizer returned an empty summary")]
  EmptySummary,
}

#[async_trait]
pub trait Summarizer: Send + Sync {
  async fn summarize(&self, messages: &[Message]) -> Result<String, SummarizeError>;
}

#[derive(Debug, Clone)]
pub struct LlmSummarizer {
  client: LlmClient,
}

impl LlmSummarizer {
  pub fn new(client: LlmClient) -> Self {
    Self { client }
  }

  fn format_messages(messages: &[Message]) -> String {
    messages
      .iter()
      .map(|message| {
        let role = match message.role {
          Role::System => "system",
          Role::User => "user",
          Role::Assistant => "assistant",
          Role::Tool => "tool",
        };

        format!("{role}: {}", message.content)
      })
      .collect::<Vec<_>>()
      .join("\n")
  }
}

#[async_trait]
impl Summarizer for LlmSummarizer {
  async fn summarize(&self, messages: &[Message]) -> Result<String, SummarizeError> {
    let transcript = Self::format_messages(messages);

    let prompt = format!(
      "\
Summarize the following agent context into one short paragraph.

Perserve the concrete facts, task state, tool results, errors, numbers, \
and other information that may be needed to continue the task.

Do not invent information. Do not include commentary about the \
summarization process.

Context to summarize:

{transcript}
"
    );

    let messages = vec![Message {
      role: Role::User,
      content: prompt,
      tool_calls: Vec::new(),
      tool_call_id: None,
    }];

    let summary = self.client.chat(messages).await?;

    let summary = summary.trim().to_string();

    if summary.is_empty() {
      return Err(SummarizeError::EmptySummary);
    }

    Ok(summary)
  }
}
