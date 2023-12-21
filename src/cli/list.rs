use crate::utils;
use anyhow::Result;

pub async fn run() -> Result<()> {
    let (daemon, join) = utils::connect_daemon().await?;

    let shares = daemon.list().await??;

    if shares.is_empty() {
        println!("The daemon doesn't have any shares.");
    } else {
        let mut shares: Vec<_> = shares.into_values().collect();
        shares.sort_unstable_by(|a, b| a.name.cmp(&b.name));

        let mut first = true;

        for share in shares {
            if first {
                first = false;
            } else {
                println!();
            }

            utils::print_share(&share);
        }
    }

    daemon.client().shutdown();
    join.await??;
    Ok(())
}
