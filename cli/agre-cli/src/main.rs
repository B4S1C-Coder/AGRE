use agre_core::{Message, Role};
use agre_llm::LlmClient;
use anyhow::{Context, Result};
use clap::Parser;

#[derive(Debug, Parser)]
#[command(name = "agre-cli")]
#[command(about = "One-shot client for the AGRE LLM Backend")]
struct Args {
  prompt: String,

  /// OpenAI compatible
  #[arg(long, default_value = "http://localhost:8080", env = "AGRE_BASE_URL")]
  base_url: String,

  #[arg(long, default_value = "local-model", env = "AGRE_MODEL")]
  model: String,
}

async fn run() -> Result<()> {
  let args = Args::parse();

  let api_key = std::env::var("AGRE_LLM_PROVIDER_API_KEY").ok();

  let client = LlmClient::new(args.base_url, args.model, api_key);

  let messages = vec![Message {
    role: Role::User,
    content: args.prompt,
    tool_calls: Vec::new(),
    tool_call_id: None,
  }];

  let response = client.chat(messages).await.context("LLM request failed")?;

  println!("{response}");

  Ok(())
}

#[tokio::main]
async fn main() {
  dotenvy::dotenv().ok();

  if let Err(error) = run().await {
    eprintln!("error: {error:#}");
    std::process::exit(1);
  }
}
