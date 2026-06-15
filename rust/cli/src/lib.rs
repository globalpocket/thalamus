use clap::Parser;

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
        }
    }
}
