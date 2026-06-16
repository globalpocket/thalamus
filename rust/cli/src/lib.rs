use clap::Parser;
use thalamus_bus::BasicBus;
use thalamus_protocol::payload::{RuntimeLLMRequestPayload, RuntimeToolRequestPayload};
use thalamus_protocol::subject::{RUNTIME_LLM_REQUEST, RUNTIME_TOOL_REQUEST};
use thalamus_runtime::ThalamusRuntime;

/// Thalamus CLI - Agent Runtime Command Line Interface
#[derive(Parser, Debug)]
#[command(name = "thalamus")]
#[command(about = "Thalamus Agent Runtime CLI")]
pub struct ThalamusCLI {
    /// Verbose output
    #[arg(short, long)]
    pub verbose: bool,

    /// Subcommand to execute
    #[command(subcommand)]
    pub command: CLICommand,
}

/// Available CLI subcommands
#[derive(clap::Subcommand, Debug)]
pub enum CLICommand {
    /// Start the runtime
    Start {
        /// Configuration file path
        #[arg(short, long, default_value = "config.yaml")]
        config: String,
    },
    /// Stop the runtime
    Stop,
    /// Show runtime status
    Status,
    /// List available agents
    ListAgents,
    /// Run the local deterministic demo
    RunDemo,
}

/// CLI Error types
#[derive(Debug, thiserror::Error)]
pub enum CliError {
    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Runtime error: {0}")]
    RuntimeError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

impl ThalamusCLI {
    /// Create a new ThalamusCLI instance from command line arguments
    pub fn new() -> Self {
        Self::parse()
    }

    /// Run the CLI command
    pub async fn run(&self) -> Result<(), CliError> {
        match &self.command {
            CLICommand::Start { config } => {
                if self.verbose {
                    eprintln!("Starting runtime with config: {}", config);
                }
                // Runtime initialization will be implemented in Unit 6
                Ok(())
            }
            CLICommand::Stop => {
                if self.verbose {
                    eprintln!("Stopping runtime");
                }
                // Runtime stop will be implemented in Unit 6
                Ok(())
            }
            CLICommand::Status => {
                if self.verbose {
                    eprintln!("Checking runtime status");
                }
                // Runtime status will be implemented in Unit 6
                Ok(())
            }
            CLICommand::ListAgents => {
                if self.verbose {
                    eprintln!("Listing available agents");
                }
                // Agent listing will be implemented in Unit 6
                Ok(())
            }
            CLICommand::RunDemo => {
                if self.verbose {
                    eprintln!("Running demo");
                }
                let mut runtime = ThalamusRuntime::new(BasicBus::new());
                runtime
                    .start()
                    .await
                    .map_err(|e| CliError::RuntimeError(e.to_string()))?;

                let llm_prompt = "summarize runtime MVP";
                let tool_input = serde_json::json!({ "text": "runtime MVP" });

                runtime
                    .publish(
                        RUNTIME_LLM_REQUEST.to_string(),
                        "thalamus-cli".to_string(),
                        serde_json::to_value(RuntimeLLMRequestPayload {
                            request_id: "llm-request-1".to_string(),
                            task_id: Some("task-runtime-1".to_string()),
                            prompt: Some(llm_prompt.to_string()),
                            messages: Vec::new(),
                            model: Some("mock-model".to_string()),
                            agent_id: Some("agent-1".to_string()),
                            correlation_id: Some("demo-correlation-1".to_string()),
                            options: serde_json::json!({}),
                            timeout_seconds: None,
                        })
                        .map_err(|e| CliError::RuntimeError(e.to_string()))?,
                    )
                    .await
                    .map_err(|e| CliError::RuntimeError(e.to_string()))?;
                runtime
                    .publish(
                        RUNTIME_TOOL_REQUEST.to_string(),
                        "thalamus-cli".to_string(),
                        serde_json::to_value(RuntimeToolRequestPayload {
                            request_id: "tool-request-1".to_string(),
                            task_id: Some("task-runtime-1".to_string()),
                            capability: "echo".to_string(),
                            input: tool_input.clone(),
                            agent_id: Some("agent-1".to_string()),
                            correlation_id: Some("demo-correlation-1".to_string()),
                            options: serde_json::json!({}),
                            timeout_seconds: None,
                        })
                        .map_err(|e| CliError::RuntimeError(e.to_string()))?,
                    )
                    .await
                    .map_err(|e| CliError::RuntimeError(e.to_string()))?;

                println!("Runtime Event Flow");
                println!("Mock response: {}", llm_prompt);
                println!("{}", tool_input);
                Ok(())
            }
        }
    }
}

impl Default for ThalamusCLI {
    fn default() -> Self {
        Self::new()
    }
}
