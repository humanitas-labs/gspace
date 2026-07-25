use clap::Parser;

#[tokio::main]
async fn main() {
    let cli = gspace::gcal::GcalCli::parse();

    if let Err(err) = gspace::gcal::run(cli).await {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}
