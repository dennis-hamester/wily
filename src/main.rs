#![forbid(unsafe_code)]

mod bus;
mod cli;
mod daemon;
mod logging;
mod schemas;
mod shutdown_notifier;
mod task_handle;
mod utils;

use anyhow::Result;
use clap::Parser;

#[derive(Debug, Parser)]
#[clap(version, about)]
enum Args {
    /// Run the daemon to share files.
    Daemon(daemon::Args),

    /// Shut down the daemon.
    ShutDown,

    /// Share a directory or file.
    Share(cli::share::Args),

    /// Remove a share.
    Unshare(cli::unshare::Args),

    /// List all shares of the local daemon.
    List,

    /// Query information about a shared file or directory.
    Query(cli::query::Args),
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    match args {
        Args::Daemon(args) => daemon::run(args).await,
        Args::ShutDown => cli::shut_down::run().await,
        Args::Share(args) => cli::share::run(args).await,
        Args::Unshare(args) => cli::unshare::run(args).await,
        Args::List => cli::list::run().await,
        Args::Query(args) => cli::query::run(args).await,
    }
}
