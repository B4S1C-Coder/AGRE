use agre_context::MemoryStore;

#[test]
fn keyword_search_finds_episode() {
  let store = MemoryStore::in_memory().unwrap();

  store.store_episode("calculator returned 42", None).unwrap();

  store
    .store_episode("HTTP request returned 404", None)
    .unwrap();

  let results = store.search_by_keyword("calculator").unwrap();

  assert_eq!(results.len(), 1);
  assert_eq!(results[0].summary_text, "calculator returned 42");
}

#[test]
fn embedding_search_orders_by_similarity() {
  let store = MemoryStore::in_memory().unwrap();

  store.store_episode("close", Some(&[1.0, 0.0])).unwrap();

  store.store_episode("far", Some(&[0.0, 1.0])).unwrap();

  let results = store.search_by_embedding(&[0.9, 0.1], 2).unwrap();

  assert_eq!(results.len(), 2);
  assert_eq!(results[0].episode.summary_text, "close");
}
