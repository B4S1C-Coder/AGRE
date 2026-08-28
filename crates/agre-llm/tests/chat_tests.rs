use agre_core::{Message, Role};
use agre_llm::{LlmClient, LlmError};
use serde_json::json;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

fn user_messages(content: &str) -> Vec<Message> {
  vec![Message {
    role: Role::User,
    content: content.to_string(),
    tool_calls: Vec::new(),
    tool_call_id: None
  }]
}

#[tokio::test]
async fn happy_path_extracts_content() {
  let server = MockServer::start().await;

  Mock::given(method("POST"))
    .and(path("/v1/chat/completions"))
    .respond_with(ResponseTemplate::new(200).set_body_json(json!({
      "choices": [
        { "message": { "content": "4" } }
      ]
    })))
    .mount(&server)
    .await;

  let client = LlmClient::new(server.uri(), "test-model", None);
  let result = client.chat(user_messages("What is 2 + 2?")).await;

  assert_eq!(result.unwrap(), "4");
}

#[tokio::test]
async fn non_2xx_status_maps_to_http_status_error() {
  let server = MockServer::start().await;

  Mock::given(method("POST"))
    .and(path("/v1/chat/completions"))
    .respond_with(ResponseTemplate::new(500).set_body_string("internal error"))
    .mount(&server)
    .await;

  let client = LlmClient::new(server.uri(), "test-model", None);
  let result = client.chat(user_messages("hello")).await;

  match result {
    Err(LlmError::HttpStatus { status, body }) => {
      assert_eq!(status, 500);
      assert_eq!(body, "internal error");
    }
    other => panic!("expected HttpStatus error, got {other:?}"),
  }
}

#[tokio::test]
async fn malformed_json_maps_to_invalid_json_error() {
  let server = MockServer::start().await;

  Mock::given(method("POST"))
    .and(path("/v1/chat/completions"))
    .respond_with(ResponseTemplate::new(200).set_body_string("{ not valid json"))
    .mount(&server)
    .await;

  let client = LlmClient::new(server.uri(), "test-model", None);
  let result = client.chat(user_messages("hello")).await;

  assert!(matches!(result, Err(LlmError::InvalidJson(_))));
}

#[tokio::test]
async fn empty_choices_maps_to_unexpected_response_error() {
  let server = MockServer::start().await;

  Mock::given(method("POST"))
    .and(path("/v1/chat/completions"))
    .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "choices": [] })))
    .mount(&server)
    .await;

  let client = LlmClient::new(server.uri(), "test-model", None);
  let result = client.chat(user_messages("hello")).await;

  assert!(matches!(result, Err(LlmError::UnexpectedResponse(_))));
}

#[tokio::test]
async fn api_key_is_sent_as_bearer_token_when_present() {
  let server = MockServer::start().await;

  Mock::given(method("POST"))
    .and(path("/v1/chat/completions"))
    .and(header("authorization", "Bearer sk-test-123"))
    .respond_with(ResponseTemplate::new(200).set_body_json(json!({
      "choices": [{ "message": { "content": "ok" } }]
    })))
    .mount(&server)
    .await;

  let client = LlmClient::new(server.uri(), "test-model", Some("sk-test-123".to_string()));
  let result = client.chat(user_messages("hello")).await;

  assert_eq!(result.unwrap(), "ok");
}

#[tokio::test]
async fn api_key_is_not_present() {
  let server = MockServer::start().await;

  Mock::given(method("POST"))
    .and(path("/v1/chat/completions"))
    .and(|req: &Request| !req.headers.contains_key("authorization"))
    .respond_with(ResponseTemplate::new(200).set_body_json(json!({
      "choices": [{ "message": { "content": "ok" } }]
    })))
    .expect(1)
    .mount(&server)
    .await;

  let client = LlmClient::new(server.uri(), "test-local-model", None);
  let result = client.chat(user_messages("hello")).await;

  assert_eq!(result.unwrap(), "ok");   
}