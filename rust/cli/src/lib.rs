use std::{future::Future, pin::Pin, sync::Arc};

use clap::Parser;
use thalamus_bus::{BasicBus, BusError, Handler, MessageBus, SubscriptionId};
use thalamus_protocol::subject::{
    RUNTIME_LLM_REQUEST, RUNTIME_LLM_RESPONSE, RUNTIME_TASK_ASSIGN, RUNTIME_TASK_RESULT,
    RUNTIME_TOOL_REQUEST, RUNTIME_TOOL_RESULT,
};
use thalamus_protocol::{
    payload::{
        RuntimeLLMRequestPayload, RuntimeTaskAssignPayload, RuntimeTaskResultPayload,
        RuntimeToolRequestPayload,
    },
    EventEnvelope,
};
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

#[derive(Clone)]
struct ObservableBus {
    inner: Arc<tokio::sync::RwLock<BasicBus>>,
}

impl ObservableBus {
    fn new() -> Self {
        Self {
            inner: Arc::new(tokio::sync::RwLock::new(BasicBus::new())),
        }
    }

    async fn published_events(&self) -> Vec<EventEnvelope> {
        self.inner.read().await.published_events().await
    }
}

impl MessageBus for ObservableBus {
    fn subscribe<'life0, 'async_trait>(
        &'life0 mut self,
        subject: String,
        handler: Handler,
    ) -> Pin<Box<dyn Future<Output = Result<SubscriptionId, BusError>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move { self.inner.write().await.subscribe(subject, handler).await })
    }

    fn publish<'life0, 'async_trait>(
        &'life0 self,
        envelope: EventEnvelope,
    ) -> Pin<Box<dyn Future<Output = Result<(), BusError>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move { self.inner.read().await.publish(envelope).await })
    }

    fn unsubscribe<'life0, 'async_trait>(
        &'life0 mut self,
        id: SubscriptionId,
    ) -> Pin<Box<dyn Future<Output = Result<(), BusError>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move { self.inner.write().await.unsubscribe(id).await })
    }

    fn close<'life0, 'async_trait>(
        &'life0 mut self,
    ) -> Pin<Box<dyn Future<Output = ()> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move { self.inner.write().await.close().await })
    }

    fn is_closed<'life0, 'async_trait>(
        &'life0 self,
    ) -> Pin<Box<dyn Future<Output = bool> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move { self.inner.read().await.is_closed().await })
    }

    fn handler_count<'life0, 'life1, 'async_trait>(
        &'life0 self,
        subject: &'life1 str,
    ) -> Pin<Box<dyn Future<Output = usize> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move { self.inner.read().await.handler_count(subject).await })
    }
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
                let bus = ObservableBus::new();
                let bus_observer = bus.clone();
                let mut runtime = ThalamusRuntime::new(bus);
                runtime
                    .start()
                    .await
                    .map_err(|e| CliError::RuntimeError(e.to_string()))?;

                let task_id = "task-runtime-1".to_string();
                let agent_id = "agent-1".to_string();
                let correlation_id = "demo-correlation-1".to_string();
                let llm_prompt = "summarize runtime MVP";
                let tool_input = serde_json::json!({ "text": "runtime MVP" });

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
                            capabilities: vec!["llm".to_string(), "echo".to_string()],
                            metadata: serde_json::json!({ "demo": true }),
                            agent_id: Some(agent_id.clone()),
                            parent_task_id: None,
                            correlation_id: Some(correlation_id.clone()),
                        })
                        .map_err(|e| CliError::RuntimeError(e.to_string()))?,
                    )
                    .await
                    .map_err(|e| CliError::RuntimeError(e.to_string()))?;
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
                runtime
                    .publish(
                        RUNTIME_TOOL_REQUEST.to_string(),
                        "thalamus-cli".to_string(),
                        serde_json::to_value(RuntimeToolRequestPayload {
                            task_id: task_id.clone(),
                            request_id: None,
                            capability: "echo".to_string(),
                            input: tool_input.clone(),
                            correlation_id: Some(correlation_id.clone()),
                            timeout_seconds: None,
                        })
                        .map_err(|e| CliError::RuntimeError(e.to_string()))?,
                    )
                    .await
                    .map_err(|e| CliError::RuntimeError(e.to_string()))?;
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
                let published_events = bus_observer.published_events().await;
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
