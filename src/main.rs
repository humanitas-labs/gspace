use clap::Parser;

#[tokio::main]
async fn main() {
    let cli = gmail::cli::Cli::parse();

    if let Err(err) = gmail::run(cli).await {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}
