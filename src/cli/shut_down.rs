use crate::utils;
use anyhow::Result;

pub async fn run() -> Result<()> {
    let (daemon, join) = utils::connect_daemon().await?;
    daemon.shut_down().await??;
    daemon.client().shutdown();
    join.await??;
    Ok(())
}
