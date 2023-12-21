use crate::schemas::{DaemonShareArgs, DaemonShareError};
use crate::utils;
use anyhow::{anyhow, Context, Result, Error};
use std::path::Path;

#[derive(Debug, clap::Args)]
pub struct Args {
    /// Path to a directory or file to share.
    path: String,

    /// Name of the share.
    ///
    /// If this is not specified, then it will be derived from the last component of the path.
    #[clap(short, long)]
    name: Option<String>,

    /// Persist the share across restarts of the daemon.
    #[clap(short, long)]
    persist: bool,

    /// Don't automatically enable the share.
    #[clap(short, long)]
    disabled: bool,
}

pub async fn run(args: Args) -> Result<()> {
    let (daemon, join) = utils::connect_daemon().await?;

    let path = utils::ensure_absolute(Path::new(&args.path))?;
    let path = path
        .to_str()
        .with_context(|| anyhow!("non UTF-8 path `{}`", path.display()))?;

    let res = daemon
        .share(&DaemonShareArgs {
            path: path.to_owned(),
            name: args.name,
            persist: Some(args.persist),
            expires_unix_ms: None,
            disabled: Some(args.disabled),
        })
        .await?;

    let res = match res {
        Ok(share) => {
            println!("Share created:");
            println!();
            utils::print_share(&share);

            Ok(())
        }

        Err(DaemonShareError::InvalidName(e)) => Err(Error::msg(e)),
        Err(DaemonShareError::DuplicateName(name)) => Err(anyhow!("duplicate share name `{name}`")),
        Err(DaemonShareError::RelativePath) => unreachable!(),
    };

    daemon.client().shutdown();
    join.await??;
    res
}
