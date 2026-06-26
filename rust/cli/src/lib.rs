use std::sync::Arc;

use clap::Parser;
use thalamus_bus::BasicBus;
use thalamus_protocol::payload::{
    RuntimeLLMRequestPayload, RuntimeTaskAssignPayload, RuntimeTaskResultPayload,
    RuntimeToolRequestPayload,
};
use thalamus_protocol::subject::{
    RUNTIME_LLM_REQUEST, RUNTIME_LLM_RESPONSE, RUNTIME_TASK_ASSIGN, RUNTIME_TASK_RESULT,
    RUNTIME_TOOL_REQUEST, RUNTIME_TOOL_RESULT,
};
use thalamus_runtime::{llm::MockLlmProvider, ThalamusRuntime};

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

                let bus = BasicBus::new();
                let mut runtime = ThalamusRuntime::new(bus, Arc::new(MockLlmProvider));
                runtime
                    .start()
                    .await
                    .map_err(|e| CliError::RuntimeError(e.to_string()))?;

                Ok(())
            }
            CLICommand::Stop => {
                if self.verbose {
                    eprintln!("Stopping runtime");
                }
                let bus = BasicBus::new();
                let mut runtime = ThalamusRuntime::new(bus, Arc::new(MockLlmProvider));
                runtime
                    .stop()
                    .await
                    .map_err(|e| CliError::RuntimeError(e.to_string()))?;
                println!("Runtime stopped");
                Ok(())
            }
            CLICommand::Status => {
                if self.verbose {
                    eprintln!("Checking runtime status");
                }
                let bus = BasicBus::new();
                let _runtime = ThalamusRuntime::new(bus, Arc::new(MockLlmProvider));
                println!("Runtime status: initialized");
                Ok(())
            }
            CLICommand::ListAgents => {
                if self.verbose {
                    eprintln!("Listing available agents");
                }
                let bus = BasicBus::new();
                let _runtime = ThalamusRuntime::new(bus, Arc::new(MockLlmProvider));
                println!("No agents registered");
                Ok(())
            }
            CLICommand::RunDemo => {
                if self.verbose {
                    eprintln!("Running demo");
                }

                let bus = BasicBus::new();
                let observer = bus.clone();
                let mut runtime = ThalamusRuntime::new(bus, Arc::new(MockLlmProvider));
                runtime
                    .start()
                    .await
                    .map_err(|e| CliError::RuntimeError(e.to_string()))?;

                let task_id = "task-runtime-1".to_string();
                let agent_id = "agent-1".to_string();
                let correlation_id = "demo-correlation-1".to_string();
                let llm_prompt = "summarize runtime MVP";
                let tool_input = serde_json::json!({ "text": "runtime MVP" });

                // Publish task.assign — internal handler creates TaskState
                runtime
                    .publish(
                        RUNTIME_TASK_ASSIGN.to_string(),
                        "thalamus-cli".to_string(),
                        serde_json::to_value(RuntimeTaskAssignPayload {
                            task_id: task_id.clone(),
                            input: serde_json::json!({
                                "prompt": llm_prompt,
                                "tool_input": tool_input,
                            }),
                            capabilities: vec!["llm".to_string(), "tool.echo".to_string()],
                            metadata: serde_json::json!({ "demo": true }),
                            agent_id: Some(agent_id.clone()),
                            parent_task_id: None,
                            correlation_id: Some(correlation_id.clone()),
                        })
                        .map_err(|e| CliError::RuntimeError(e.to_string()))?,
                    )
                    .await
                    .map_err(|e| CliError::RuntimeError(e.to_string()))?;

                // Publish llm.request — internal handler calls provider and publishes llm.response
                runtime
                    .publish(
                        RUNTIME_LLM_REQUEST.to_string(),
                        "thalamus-cli".to_string(),
                        serde_json::to_value(RuntimeLLMRequestPayload {
                            task_id: task_id.clone(),
                            request_id: None,
                            prompt: Some(llm_prompt.to_string()),
                            messages: Vec::new(),
                            model: Some("mock-model".to_string()),
                            correlation_id: Some(correlation_id.clone()),
                            options: serde_json::json!({}),
                        })
                        .map_err(|e| CliError::RuntimeError(e.to_string()))?,
                    )
                    .await
                    .map_err(|e| CliError::RuntimeError(e.to_string()))?;

                // Publish tool.request — internal handler invokes tool and publishes tool.result
                runtime
                    .publish(
                        RUNTIME_TOOL_REQUEST.to_string(),
                        "thalamus-cli".to_string(),
                        serde_json::to_value(RuntimeToolRequestPayload {
                            task_id: task_id.clone(),
                            request_id: None,
                            capability: "tool.echo".to_string(),
                            input: tool_input.clone(),
                            correlation_id: Some(correlation_id.clone()),
                            timeout_seconds: None,
                        })
                        .map_err(|e| CliError::RuntimeError(e.to_string()))?,
                    )
                    .await
                    .map_err(|e| CliError::RuntimeError(e.to_string()))?;

                // Publish task.result — internal handler updates TaskState
                runtime
                    .publish(
                        RUNTIME_TASK_RESULT.to_string(),
                        "thalamus-cli".to_string(),
                        serde_json::to_value(RuntimeTaskResultPayload {
                            task_id,
                            status: "completed".to_string(),
                            result: Some(serde_json::json!({ "demo": "runtime event flow" })),
                            error: serde_json::Value::Null,
                            correlation_id: Some(correlation_id),
                        })
                        .map_err(|e| CliError::RuntimeError(e.to_string()))?,
                    )
                    .await
                    .map_err(|e| CliError::RuntimeError(e.to_string()))?;

                println!("Runtime Event Flow");
                let published_events = observer.published_events().await;
                for event in &published_events {
                    println!("{} {}", event.subject, event.source);
                }
                for event in &published_events {
                    match event.subject.as_str() {
                        RUNTIME_LLM_RESPONSE => {
                            if let Some(content) = event.payload["message"]["content"].as_str() {
                                println!("{content}");
                            }
                        }
                        RUNTIME_TOOL_RESULT => {
                            if let Some(result) = event.payload.get("result") {
                                println!("{result}");
                            }
                        }
                        _ => {}
                    }
                }

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
