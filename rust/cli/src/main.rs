#[tokio::main]
async fn main() {
    let cli = thalamus_cli::ThalamusCLI::new();

    if let Err(error) = cli.run().await {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
