use agre_llm::LlmClient;
use agre_runtime::{AgentRuntime, RuntimeError, ToolRegistry};
use agre_tools::{Calculator, FileTool};
use serde_json::{Value, json};
use std::sync::atomic::{AtomicUsize, Ordering};
use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

///Serves a fixed seq of responses, 1/call, then repeats last one
/// indefinitely. Let's us test what model said on turn1, 2, etc.
struct Sequenced {
  responses: Vec<ResponseTemplate>,
  calls: AtomicUsize,
}

impl Sequenced {
  fn new(responses: Vec<ResponseTemplate>) -> Self {
    Self {
      responses,
      calls: AtomicUsize::new(0),
    }
  }
}

impl Respond for Sequenced {
  fn respond(&self, _req: &Request) -> ResponseTemplate {
    let index = self.calls.fetch_add(1, Ordering::SeqCst);
    self.responses[index.min(self.responses.len() - 1)].clone()
  }
}

fn tool_call_response(content: &str, calls: &[(&str, &str, &str)]) -> ResponseTemplate {
  let tool_calls: Vec<Value> = calls
    .iter()
    .map(|(id, name, args)| {
      json!({
        "id": id,
        "type": "function",
        "function": { "name": name, "arguments": args }
      })
    })
    .collect();

  ResponseTemplate::new(200).set_body_json(json!({
    "choices": [{
      "message": { "content": content, "tool_calls": tool_calls }
    }]
  }))
}

fn final_answer_response(content: &str) -> ResponseTemplate {
  ResponseTemplate::new(200).set_body_json(json!({
    "choices": [{ "message": { "content": content, "tool_calls": [] } }]
  }))
}

async fn mount_sequenced(server: &MockServer, responses: Vec<ResponseTemplate>) {
  Mock::given(wiremock::matchers::method("POST"))
    .and(wiremock::matchers::path("/v1/chat/completions"))
    .respond_with(Sequenced::new(responses))
    .mount(server)
    .await;
}

fn request_bodies(requests: &[Request]) -> Vec<Value> {
  requests
    .iter()
    .map(|r| serde_json::from_slice(&r.body).expect("request body should be valid JSON"))
    .collect()
}

#[tokio::test]
async fn happy_path_two_real_tools_end_to_end() {
  let server = MockServer::start().await;
  let workdir = tempfile::tempdir().unwrap();

  mount_sequenced(
    &server,
    vec![
      tool_call_response(
        "Calculating the calculation and saving the result...",
        &[
          ("call_calc", "calculator", r#"{"expression":"10 * 5"}"#),
          (
            "call_file",
            "file",
            r#"{"operation":"write","path":"out.txt","content":"fifty"}"#,
          ),
        ],
      ),
      final_answer_response("Result is 50 and saved in out.txt"),
    ],
  )
  .await;

  let mut registry = ToolRegistry::new();
  registry.register(Calculator).unwrap();
  registry
    .register(FileTool::new(workdir.path()).unwrap())
    .unwrap();

  let client = LlmClient::new(server.uri(), "test-model", None);
  let runtime = AgentRuntime::new(5);

  let result = runtime
    .run(&client, "system prompt", "do the task", &registry)
    .await
    .expect("agent run should succeed");

  assert_eq!(result, "Result is 50 and saved in out.txt");

  // verify tool actually ran
  let written = std::fs::read_to_string(workdir.path().join("out.txt")).unwrap();
  assert_eq!(written, "fifty");

  // verify calculator's output actually went to second request
  let requests = server.received_requests().await.unwrap();
  let bodies = request_bodies(&requests);
  let second_req = &bodies[1];

  let tool_messages = second_req["messages"]
    .as_array()
    .unwrap()
    .iter()
    .filter(|m| m["role"] == "tool")
    .collect::<Vec<_>>();

  assert!(
    tool_messages
      .iter()
      .any(|m| m["content"].as_str().unwrap().contains("50"))
  );
}

#[tokio::test]
async fn malformed_tool_json_triggers_repair_observation() {
  let server = MockServer::start().await;

  mount_sequenced(
    &server,
    vec![
      tool_call_response(
        "Calling calculator.",
        &[("call1", "calculator", r#"{"expression": not valid}"#)],
      ),
      final_answer_response("Sorry, I couldn't complete the calculation."),
    ],
  )
  .await;

  let mut registry = ToolRegistry::new();
  registry.register(Calculator).unwrap();

  let client = LlmClient::new(server.uri(), "test-model", None);
  let runtime = AgentRuntime::new(5);

  let result = runtime
    .run(&client, "system", "task", &registry)
    .await
    .expect("agent should recover via repair turn, not error out");

  assert_eq!(result, "Sorry, I couldn't complete the calculation.");

  let requests = server.received_requests().await.unwrap();
  assert_eq!(
    requests.len(),
    2,
    "repair turn should cost exactly one retry"
  );

  let bodies = request_bodies(&requests);
  let repair_message = bodies[1]["messages"]
    .as_array()
    .unwrap()
    .iter()
    .find(|m| m["tool_call_id"] == "call1")
    .expect("repair observation for call1 should be present");

  let content = repair_message["content"].as_str().unwrap();
  assert!(content.contains("malformed JSON"), "content was: {content}");
  assert!(content.contains("retry"), "content was: {content}");
}

#[tokio::test]
async fn unknown_tool_name_produces_error_observation_not_panic() {
  let server = MockServer::start().await;

  mount_sequenced(
    &server,
    vec![
      tool_call_response(
        "Calling a tool that isn't registered.",
        &[("call1", "does_not_exist", "{}")],
      ),
      final_answer_response("That tool wasn't available."),
    ],
  )
  .await;

  let registry = ToolRegistry::new();

  let client = LlmClient::new(server.uri(), "test-model", None);
  let runtime = AgentRuntime::new(5);

  let result = runtime
    .run(&client, "system", "task", &registry)
    .await
    .expect("unknown tool should be reported as an observation, not fail the run");

  assert_eq!(result, "That tool wasn't available.");

  let requests = server.received_requests().await.unwrap();
  let bodies = request_bodies(&requests);

  let observation = bodies[1]["messages"]
    .as_array()
    .unwrap()
    .iter()
    .find(|m| m["tool_call_id"] == "call1")
    .unwrap();

  let content = observation["content"].as_str().unwrap();
  assert!(content.contains("unknown tool"), "content was: {content}");
}

#[tokio::test]
async fn real_tool_execution_error_produces_error_observation() {
  let server = MockServer::start().await;

  mount_sequenced(
    &server,
    vec![
      tool_call_response(
        "Dividing by zero on purpose.",
        &[("call1", "calculator", r#"{"expression":"1 / 0"}"#)],
      ),
      final_answer_response("That calculation isn't possible."),
    ],
  )
  .await;

  let mut registry = ToolRegistry::new();
  registry.register(Calculator).unwrap();

  let client = LlmClient::new(server.uri(), "test-model", None);
  let runtime = AgentRuntime::new(5);

  let result = runtime
    .run(&client, "system", "task", &registry)
    .await
    .expect("a real tool error should still be a recoverable observation");

  assert_eq!(result, "That calculation isn't possible.");

  // let requests = server.received_requests().await.unwrap();
  // let bodies = request_bodies(&requests);

  // let observation = bodies[1]["message"]
  //   .as_array()
  //   .unwrap()
  //   .iter()
  //   .find(|m| m["tool_call_id"] == "call1")
  //   .unwrap();

  // assert!(observation["content"].as_str().unwrap().to_lowercase().contains("division"));

  let requests = server
    .received_requests()
    .await
    .expect("request recording should be enabled");

  assert_eq!(
    requests.len(),
    2,
    "expected exactly 2 LLM calls, got {}",
    requests.len()
  );

  let bodies = request_bodies(&requests);
  let second = &bodies[1];

  let messages = second
    .get("messages")
    .and_then(Value::as_array)
    .unwrap_or_else(|| {
      panic!("second request body had no 'message' array.\nFull body was:\n{second:#}")
    });

  let observation = messages
    .iter()
    .find(|m| m.get("tool_call_id").and_then(Value::as_str) == Some("call1"))
    .unwrap_or_else(|| {
      panic!("no message with tool_call_id 'call1' found.\nMessages were:\n{messages:#?}")
    });

  assert!(
    observation["content"]
      .as_str()
      .unwrap()
      .to_lowercase()
      .contains("division")
  );
}

#[tokio::test]
async fn max_iterations_bound_is_respected() {
  let server = MockServer::start().await;

  Mock::given(wiremock::matchers::method("POST"))
    .and(wiremock::matchers::path("/v1/chat/completions"))
    .respond_with(tool_call_response(
      "Still working ...",
      &[("call_x", "calculator", r#"{"expression":"4 + 69"}"#)],
    ))
    .mount(&server)
    .await;

  let mut registry = ToolRegistry::new();
  registry.register(Calculator).unwrap();

  let client = LlmClient::new(server.uri(), "test-model", None);
  let runtime = AgentRuntime::new(3);

  let result = runtime.run(&client, "system", "task", &registry).await;

  assert!(matches!(result, Err(RuntimeError::MaxIterations)));

  let requests = server.received_requests().await.unwrap();
  assert_eq!(
    requests.len(),
    3,
    "should stop at exactly max_iterations LLM calls, not run forever"
  );
}
