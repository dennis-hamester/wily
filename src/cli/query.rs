use crate::schemas::WilyQueryArgs;
use crate::utils;
use anyhow::{anyhow, Context, Error, Result};
use url::Url;

#[derive(Debug, clap::Args)]
pub struct Args {
    /// URL of a shared filed or directory.
    url: Url,
}

pub async fn run(args: Args) -> Result<()> {
    let (wily, join) = utils::connect_wily(&args.url).await?;

    dbg!(&args.url);

    let res = wily
        .query(&WilyQueryArgs {
            path: args.url.path().to_owned(),
        })
        .await?;

    dbg!(res);

    wily.client().shutdown();
    join.await??;
    Ok(())
}
