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
