use crate::schemas::{DaemonUnshareArgs, DaemonUnshareError};
use crate::utils;
use anyhow::{anyhow, Result};

#[derive(Debug, clap::Args)]
pub struct Args {
    /// Name of the share to remove.
    name: String,
}

pub async fn run(args: Args) -> Result<()> {
    let (daemon, join) = utils::connect_daemon().await?;

    let res = daemon
        .unshare(&DaemonUnshareArgs {
            name: args.name.clone(),
        })
        .await?;

    let res = match res {
        Ok(share) => {
            println!("Share `{}` remove:", args.name);
            println!();
            utils::print_share(&share);

            Ok(())
        }

        Err(DaemonUnshareError::UnknownShare) => Err(anyhow!("unknown share `{}`", args.name)),

        Err(DaemonUnshareError::StaticShare) => {
            Err(anyhow!("cannot remove static share `{}`", args.name))
        }
    };

    daemon.client().shutdown();
    join.await??;
    res
}
