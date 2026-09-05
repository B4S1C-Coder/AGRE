use crate::TokenCounter;
use agre_core::{Message, Role};

/// A view of the message supplied to an eviction strategy.
///
/// Strategy receives references because eviction is a policy decision.
/// ContextWindow would remove the actual messages so it own them.
#[derive(Debug, Clone, Copy)]
pub struct EvictableMessage<'a> {
  pub message: &'a Message,
  pub position: usize,
}

pub trait EvictionStrategy: Send + Sync {
  /// Return the positions of messages that should be evicted in the next compaction pass.
  ///
  /// Strategies can return multiple positions, although the built-in ones evict only one
  /// message at a time. Evicting incrementally keeps the amount being summarized small
  /// and makes the resulting context easier to reason about.
  fn select(&self, messages: &[EvictableMessage<'_>], counter: &dyn TokenCounter) -> Vec<usize>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Fifo;

impl EvictionStrategy for Fifo {
  fn select(&self, messages: &[EvictableMessage<'_>], _counter: &dyn TokenCounter) -> Vec<usize> {
    messages
      .first()
      .map(|msg| vec![msg.position])
      .unwrap_or_default()
  }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct RecencyWeighted;

impl EvictionStrategy for RecencyWeighted {
  fn select(&self, messages: &[EvictableMessage<'_>], _counter: &dyn TokenCounter) -> Vec<usize> {
    if messages.is_empty() {
      return Vec::new();
    }

    // This should be replaced by a learned relevance model. This is follows:
    // "recent is probably relevant"
    let oldest = messages
      .iter()
      .enumerate()
      .max_by_key(|(index, _)| messages.len() - *index)
      .map(|(_, msg)| msg.position);

    oldest.map(|pos| vec![pos]).unwrap_or_default()
  }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ImportanceScored;

impl ImportanceScored {
  fn is_tool_error(message: &Message) -> bool {
    if message.role != Role::Tool {
      return false;
    }

    let content = message.content.to_ascii_lowercase();

    [
      "error",
      "failed",
      "failure",
      "malformed",
      "unknown tool",
      "division by zero",
    ]
    .iter()
    .any(|marker| content.contains(marker))
  }

  fn importance(message: &Message) -> f64 {
    let mut importance = 1.0;

    // Tool errors are protected, because a forgotten error might result in
    // performing the failed action again.
    if Self::is_tool_error(message) {
      importance *= 3.0;
    }

    // numeric observations etc. represent concrete task state as opposed
    // to semantic understanding.
    if message.content.chars().any(|c| c.is_ascii_digit()) {
      importance *= 1.5;
    }

    importance
  }
}

impl EvictionStrategy for ImportanceScored {
  fn select(&self, messages: &[EvictableMessage<'_>], counter: &dyn TokenCounter) -> Vec<usize> {
    if messages.is_empty() {
      return Vec::new();
    }

    let length = messages.len();

    let selected = messages
      .iter()
      .enumerate()
      .max_by(|(left_index, left), (right_index, right)| {
        let left_age = (length - *left_index) as f64;
        let right_age = (length - *right_index) as f64;

        let left_importance = Self::importance(left.message);
        let right_importance = Self::importance(right.message);

        // token count is tie-breaker
        let left_score =
          (left_age / left_importance) * counter.count(&left.message.content).max(1) as f64;

        let right_score =
          (right_age / right_importance) * counter.count(&right.message.content).max(1) as f64;

        left_score
          .partial_cmp(&right_score)
          .unwrap_or(std::cmp::Ordering::Equal)
      })
      .map(|(_, message)| message.position);

    selected.map(|pos| vec![pos]).unwrap_or_default()
  }
}
