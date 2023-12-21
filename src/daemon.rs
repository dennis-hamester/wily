mod daemon_calls;
mod private_bus;
mod public_bus;
mod wily_calls;

use crate::logging::Logging;
use crate::schemas::{Daemon, Share, Wily, DAEMON_OBJECT_UUID, WILY_OBJECT_UUID};
use aldrin::Object;
use anyhow::Result;
use parking_lot::RwLock;
use private_bus::PrivateBus;
use public_bus::PublicBus;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::signal::unix::{signal, Signal, SignalKind};

#[derive(Debug, clap::Args)]
pub struct Args {
    #[clap(flatten)]
    logging: Logging,
}

pub async fn run(args: Args) -> Result<()> {
    args.logging.init();
    Mainloop::new(args).await?.run().await
}

struct Mainloop {
    shutdown: bool,
    public_bus: PublicBus,
    private_bus: PrivateBus,
    sigint: Signal,
    sigterm: Signal,
    _wily_obj: Object,
    wily: Wily,
    _daemon_obj: Object,
    daemon: Daemon,
    shares: Arc<RwLock<HashMap<String, Share>>>,
}

impl Mainloop {
    async fn new(_args: Args) -> Result<Self> {
        log::info!("Starting daemon.");

        let public_bus = PublicBus::new().await?;
        let private_bus = PrivateBus::new().await?;

        let sigint = signal(SignalKind::interrupt())?;
        let sigterm = signal(SignalKind::terminate())?;

        let wily_obj = public_bus.create_object(WILY_OBJECT_UUID).await?;
        let wily = Wily::new(&wily_obj).await?;

        let daemon_obj = private_bus.create_object(DAEMON_OBJECT_UUID).await?;
        let daemon = Daemon::new(&daemon_obj).await?;

        Ok(Self {
            shutdown: false,
            public_bus,
            private_bus,
            sigint,
            sigterm,
            _wily_obj: wily_obj,
            wily,
            _daemon_obj: daemon_obj,
            daemon,
            shares: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    async fn run(mut self) -> Result<()> {
        log::info!("Entering mainloop.");

        while !self.shutdown {
            tokio::select! {
                Some(call) = self.wily.next_call() => {
                    match call {
                        Ok(call) => self.wily_call(call),
                        Err(e) => log::error!("Received invalid call on the public bus: {e}."),
                    }
                }

                Some(call) = self.daemon.next_call() => {
                    match call {
                        Ok(call) => self.daemon_call(call).await?,
                        Err(e) => log::error!("Received invalid call on the private bus: {e}."),
                    }
                }

                () = self.public_bus.wait() => {
                    log::error!("Public bus shut down unexpectedly.");
                    break;
                }

                () = self.private_bus.wait() => {
                    log::error!("Private bus shut down unexpectedly.");
                    break;
                }

                signal = self.sigint.recv() => {
                    signal.unwrap();
                    log::info!("SIGINT received.");
                    break;
                }

                signal = self.sigterm.recv() => {
                    signal.unwrap();
                    log::info!("SIGTERM received.");
                    break;
                }
            }
        }

        log::info!("Shutting down.");
        self.public_bus.shutdown().await?;
        self.private_bus.shutdown().await?;

        Ok(())
    }
}
