use agre_llm::LlmClient;
use agre_runtime::{AgentRuntime, ToolRegistry};
use agre_tools::{Calculator, DeleteFileTool, FileTool, HttpFetch};
use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "agre-cli-v2")]
#[command(about = "Agent Guardrail & Runtime Engine v2 (tool calling support)")]
struct Args {
  prompt: String,

  #[arg(long, default_value = "http://localhost:8080", env = "AGRE_BASE_URL")]
  base_url: String,

  #[arg(long, default_value = "local-model", env = "AGRE_MODEL")]
  model: String,

  #[arg(long, default_value = ".", env = "AGRE_WORKING_DIRECTORY")]
  working_directory: PathBuf,

  #[arg(long, default_value_t = 10, env = "AGRE_MAX_ITERATIONS")]
  max_iterations: usize,
}

async fn run() -> Result<()> {
  let args = Args::parse();

  let api_key = std::env::var("AGRE_LLM_PROVIDER_API_KEY").ok();
  let client = LlmClient::new(args.base_url, args.model, api_key);

  let mut registry = ToolRegistry::new();

  registry
    .register(Calculator)
    .context("failed to register Calculator tool.")?;
  registry
    .register(HttpFetch::new())
    .context("failed to register HttpFetch tool.")?;

  registry
    .register(FileTool::new(&args.working_directory).context("failed to create file tool")?)
    .context("failed to register file tool.")?;

  registry
    .register(
      DeleteFileTool::new(&args.working_directory).context("failed to create delete-file tool.")?,
    )
    .context("failed to register delete-file tool.")?;

  let runtime = AgentRuntime::new(args.max_iterations);

  let system_prompt = "\
You are an agent operating through AGRE.

Use the provided tools when they are useful for completing the task.
Do not claim that an action was performed unless the corresponding
tool actually succeeded.
";

  let answer = runtime
    .run(&client, system_prompt, &args.prompt, &registry)
    .await
    .context("agent execution failed")?;

  println!("{answer}");

  Ok(())
}

#[tokio::main]
async fn main() {
  dotenvy::dotenv().ok();

  if let Err(error) = run().await {
    eprint!("error: {error:#}");
    std::process::exit(1);
  }
}
