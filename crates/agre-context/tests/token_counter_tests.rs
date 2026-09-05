use agre_context::{HeuristicTokenCounter, TokenCounter};

#[test]
fn empty_text_has_zero_tokens() {
  let counter = HeuristicTokenCounter;
  assert_eq!(counter.count(""), 0);
}

#[test]
fn four_chars_are_one_token() {
  let counter = HeuristicTokenCounter;
  assert_eq!(counter.count("text"), 1);
}

#[test]
fn partial_token_rounds_up() {
  let counter = HeuristicTokenCounter;
  assert_eq!(counter.count("texts"), 2);
}

#[test]
fn counts_unicode_chars_not_utf8_bytes() {
  let counter = HeuristicTokenCounter;
  assert_eq!(counter.count("नमस्ते"), 2);
}
