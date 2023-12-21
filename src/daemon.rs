mod public_bus;

use aldrin_broker::Broker;
use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use public_bus::PublicBus;

#[derive(Debug, Parser)]
pub struct Args {}

pub async fn run(args: Args) -> Result<()> {
    env_logger::init();
    Mainloop::new(args).await?.run().await
}

struct Mainloop {
    public_bus: PublicBus,
}

impl Mainloop {
    async fn new(args: Args) -> Result<Self> {
        log::info!("Starting daemon.");

        let public_bus = PublicBus::new().await?;

        Ok(Self { public_bus })
    }

    async fn run(mut self) -> Result<()> {
        todo!()
    }
}
