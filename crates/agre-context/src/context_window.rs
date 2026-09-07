use agre_core::Message;
use thiserror::Error;

use crate::{EvictableMessage, EvictionStrategy, SummarizeError, Summarizer, TokenCounter};

#[derive(Debug, Error)]
pub enum ContextError {
  #[error("context cannot be compacted any further while remaining over budget")]
  CannotCompactFurther,

  #[error("failed to summarize evicted context: {0}")]
  Summarization(#[from] SummarizeError),
}

#[derive(Debug, Clone)]
struct ContextEntry {
  message: Message,
  order: u64,
}

pub struct ContextWindow {
  token_budget: usize,
  token_counter: Box<dyn TokenCounter>,
  eviction_strategy: Box<dyn EvictionStrategy>,
  summarizer: Box<dyn Summarizer>,
  pinned: Vec<ContextEntry>,
  evictable: Vec<ContextEntry>,
  next_order: u64,
}

impl ContextWindow {
  pub fn new(
    token_budget: usize,
    token_counter: Box<dyn TokenCounter>,
    eviction_strategy: Box<dyn EvictionStrategy>,
    summarizer: Box<dyn Summarizer>,
  ) -> Self {
    Self {
      token_budget,
      token_counter,
      eviction_strategy,
      summarizer,
      pinned: Vec::new(),
      evictable: Vec::new(),
      next_order: 0,
    }
  }

  pub fn token_budget(&self) -> usize {
    self.token_budget
  }

  pub fn add_pinned_message(&mut self, message: Message) {
    let entry = self.new_entry(message);
    self.pinned.push(entry);
  }

  pub fn add_evictable_message(&mut self, message: Message) {
    let entry = self.new_entry(message);
    self.evictable.push(entry);
  }

  pub fn token_count(&self) -> usize {
    self
      .pinned
      .iter()
      .chain(self.evictable.iter())
      .map(|entry| self.message_token_count(&entry.message))
      .sum()
  }

  pub fn messages(&self) -> Vec<Message> {
    let mut entries = self
      .pinned
      .iter()
      .chain(self.evictable.iter())
      .collect::<Vec<_>>();

    entries.sort_by_key(|entry| entry.order);

    entries
      .into_iter()
      .map(|entry| entry.message.clone())
      .collect()
  }

  /// Compact until it fits within budget.
  ///
  /// Successful Compaction => Token count strictly decreases
  ///
  /// If no evictable messages i.e. strategy can't select anything
  /// or generated summary does not reduce token usage, compaction
  /// stops with CannotCompactFurther.
  ///
  /// Therefore, this loop is bounded and cannot run forever.
  pub async fn ensure_within_budget(&mut self) -> Result<Vec<String>, ContextError> {
    let mut summaries = Vec::new();

    loop {
      let before_tokens = self.token_count();

      if before_tokens <= self.token_budget {
        return Ok(summaries);
      }

      if self.evictable.is_empty() {
        return Err(ContextError::CannotCompactFurther);
      }

      let candidates = self
        .evictable
        .iter()
        .enumerate()
        .map(|(position, entry)| EvictableMessage {
          message: &entry.message,
          position,
        })
        .collect::<Vec<_>>();

      let selected_positions = self
        .eviction_strategy
        .select(&candidates, self.token_counter.as_ref());

      if selected_positions.is_empty() {
        return Err(ContextError::CannotCompactFurther);
      }

      if selected_positions
        .iter()
        .any(|pos| *pos >= self.evictable.len())
      {
        return Err(ContextError::CannotCompactFurther);
      }

      let mut selected_positions = selected_positions;
      selected_positions.sort_unstable();
      selected_positions.dedup();

      let selected_messages = selected_positions
        .iter()
        .map(|pos| self.evictable[*pos].message.clone())
        .collect::<Vec<_>>();

      let summary = self.summarizer.summarize(&selected_messages).await?;

      let summary_token_count = self.token_counter.count(&summary);
      let selected_token_count = selected_messages
        .iter()
        .map(|msg| self.message_token_count(msg))
        .sum::<usize>();

      if summary_token_count >= selected_token_count {
        return Err(ContextError::CannotCompactFurther);
      }

      let summary_order = selected_positions
        .iter()
        .map(|pos| self.evictable[*pos].order)
        .min()
        .unwrap_or(self.next_order);

      for position in selected_positions.into_iter().rev() {
        self.evictable.remove(position);
      }

      self.evictable.push(ContextEntry {
        message: Message {
          role: agre_core::Role::Assistant,
          content: summary.clone(),
          tool_calls: Vec::new(),
          tool_call_id: None,
        },
        order: summary_order,
      });

      self.next_order = self.next_order.max(summary_order + 1);

      let after_tokens = self.token_count();

      if after_tokens >= before_tokens {
        return Err(ContextError::CannotCompactFurther);
      }

      summaries.push(summary);
    }
  }

  fn new_entry(&mut self, message: Message) -> ContextEntry {
    let order = self.next_order;
    self.next_order += 1;

    ContextEntry { message, order }
  }

  fn message_token_count(&self, message: &Message) -> usize {
    let mut text = message.content.clone();

    for tool_call in &message.tool_calls {
      text.push_str(&tool_call.id);
      text.push_str(&tool_call.name);
      text.push_str(&tool_call.arguments);
    }

    if let Some(tool_call_id) = &message.tool_call_id {
      text.push_str(tool_call_id);
    }

    self.token_counter.count(&text)
  }
}
