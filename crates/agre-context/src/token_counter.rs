pub trait TokenCounter: Send + Sync {
  fn count(&self, text: &str) -> usize;
}

/// Intentionally approximates BPE Tokenization at ~ 4 chars per token.
/// This estimate is good enough to implement context budgeting.
///
/// >**Note**: TokenCounter trait isolates this approximation from rest
/// > of the context system so a real tokenizer implementation can be put
/// > in place without disturbing the rest of the system.
#[derive(Debug, Clone, Copy, Default)]
pub struct HeuristicTokenCounter;

impl TokenCounter for HeuristicTokenCounter {
  fn count(&self, text: &str) -> usize {
    if text.is_empty() {
      return 0;
    }

    text.chars().count().div_ceil(4)
  }
}
