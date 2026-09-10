use agre_context::{
  ContextError, ContextWindow, Fifo, HeuristicTokenCounter, SummarizeError, Summarizer,
};
use agre_core::{Message, Role};
use async_trait::async_trait;

struct FixedSummarizer {
  summary: String,
}

#[async_trait]
impl Summarizer for FixedSummarizer {
  async fn summarize(&self, _messages: &[Message]) -> Result<String, SummarizeError> {
    Ok(self.summary.clone())
  }
}

fn user_message(content: &str) -> Message {
  Message {
    role: Role::User,
    content: content.to_string(),
    tool_calls: Vec::new(),
    tool_call_id: None,
  }
}

#[tokio::test]
async fn pinned_messages_survive_eviction() {
  let mut window = ContextWindow::new(
    10,
    Box::new(HeuristicTokenCounter),
    Box::new(Fifo),
    Box::new(FixedSummarizer {
      summary: "short".to_string(),
    }),
  );

  window.add_pinned_message(user_message("important constraint"));
  window.add_evictable_message(user_message("this is old context that should disappear"));

  window
    .ensure_within_budget()
    .await
    .expect("context should compact");

  let messages = window.messages();
  assert!(messages.iter().any(|m| m.content == "important constraint"));
}

#[tokio::test]
async fn no_progress_returns_cannot_compact_further() {
  let mut window = ContextWindow::new(
    1,
    Box::new(HeuristicTokenCounter),
    Box::new(Fifo),
    Box::new(FixedSummarizer {
      summary: "1234".to_string(),
    }),
  );

  window.add_pinned_message(user_message("pinned"));
  window.add_evictable_message(user_message("1234"));

  let result = window.ensure_within_budget().await;
  assert!(matches!(result, Err(ContextError::CannotCompactFurther)));
}

#[tokio::test]
async fn pinned_context_alone_can_make_window_uncompactable() {
  let mut window = ContextWindow::new(
    1,
    Box::new(HeuristicTokenCounter),
    Box::new(Fifo),
    Box::new(FixedSummarizer {
      summary: "short".to_string(),
    }),
  );

  window.add_pinned_message(user_message("this cannot fit"));

  let result = window.ensure_within_budget().await;

  assert!(matches!(result, Err(ContextError::CannotCompactFurther)));
}
