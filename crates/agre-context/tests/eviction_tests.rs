use agre_context::{
  EvictableMessage, EvictionStrategy, Fifo, HeuristicTokenCounter, ImportanceScored,
  RecencyWeighted,
};
use agre_core::{Message, Role};

fn message(role: Role, content: &str) -> Message {
  Message {
    role,
    content: content.to_string(),
    tool_calls: Vec::new(),
    tool_call_id: None,
  }
}

#[test]
fn fifo_selects_oldest() {
  let messages = [
    message(Role::User, "old"),
    message(Role::Assistant, "middle"),
    message(Role::Tool, "new"),
  ];

  let candidates = messages
    .iter()
    .enumerate()
    .map(|(position, message)| EvictableMessage { message, position })
    .collect::<Vec<_>>();

  let selected = Fifo.select(&candidates, &HeuristicTokenCounter);
  assert_eq!(selected, vec![0]);
}

#[test]
fn recency_weighted_prefers_oldest() {
  let messages = [
    message(Role::User, "old"),
    message(Role::Assistant, "middle"),
    message(Role::Tool, "new"),
  ];

  let candidates = messages
    .iter()
    .enumerate()
    .map(|(position, message)| EvictableMessage { message, position })
    .collect::<Vec<_>>();

  let selected = RecencyWeighted.select(&candidates, &HeuristicTokenCounter);

  assert_eq!(selected, vec![0]);
}

#[test]
fn importance_score_protects_tool_error() {
  let messages = [
    message(Role::Tool, "tool failed with error"),
    message(Role::User, "ordinary conversation"),
  ];

  let candidates = messages
    .iter()
    .enumerate()
    .map(|(position, message)| EvictableMessage { message, position })
    .collect::<Vec<_>>();

  let selected = ImportanceScored.select(&candidates, &HeuristicTokenCounter);

  assert_eq!(selected, vec![1]);
}
