use clap::Parser;
use std::process::Command;
use thalamus_cli::{CLICommand, ThalamusCLI};

#[test]
fn contract_parses_status_subcommand() {
    let cli = ThalamusCLI::parse_from(["thalamus", "status"]);

    assert!(!cli.verbose);
    assert!(matches!(cli.command, CLICommand::Status));
}

#[test]
fn contract_parses_verbose_start_with_custom_config() {
    let cli = ThalamusCLI::parse_from([
        "thalamus",
        "--verbose",
        "start",
        "--config",
        "custom.yaml",
    ]);

    assert!(cli.verbose);
    match cli.command {
        CLICommand::Start { config } => assert_eq!(config, "custom.yaml"),
        other => panic!("expected start command, got {other:?}"),
    }
}

#[tokio::test]
async fn contract_parses_list_agents_subcommand() {
    let cli = ThalamusCLI::parse_from(["thalamus", "list-agents"]);

    assert!(!cli.verbose);
    assert!(matches!(cli.command, CLICommand::ListAgents));
    assert!(cli.run().await.is_ok());
}

#[tokio::test]
async fn behavior_run_status_subcommand_completes_successfully() {
    let cli = ThalamusCLI::parse_from(["thalamus", "--verbose", "status"]);

    assert!(cli.run().await.is_ok());
}

#[tokio::test]
async fn behavior_run_stop_subcommand_completes_successfully() {
    let cli = ThalamusCLI::parse_from(["thalamus", "--verbose", "stop"]);

    assert!(cli.run().await.is_ok());
}

#[tokio::test]
async fn behavior_run_start_with_custom_config_completes_successfully() {
    let cli = ThalamusCLI::parse_from([
        "thalamus",
        "--verbose",
        "start",
        "--config",
        "custom.yaml",
    ]);

    assert!(cli.run().await.is_ok());
}

#[test]
fn behavior_binary_verbose_list_agents_uses_entrypoint() {
    let output = Command::new(env!("CARGO_BIN_EXE_thalamus-cli"))
        .args(["--verbose", "list-agents"])
        .output()
        .expect("failed to run thalamus-cli binary");

    assert!(
        output.status.success(),
        "expected thalamus-cli to exit successfully, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Listing available agents"),
        "expected stderr to contain verbose list-agents message, got: {stderr}"
    );
}
