mod daemon;
mod schema;

use anyhow::Result;
use clap::Parser;

#[derive(Debug, Parser)]
#[clap(about)]
enum Args {
    /// Run the daemon to share local files.
    Daemon(daemon::Args),
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    match args {
        Args::Daemon(args) => daemon::run(args).await,
    }
}
