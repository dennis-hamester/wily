use aldrin_broker::{Broker, BrokerHandle};
use anyhow::{anyhow, Context, Result};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Notify;
use tokio::task::JoinHandle;

const BIND: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 9999);

pub struct PublicBus {
    shutdown: Arc<Notify>,
    join: JoinHandle<Result<()>>,
}

impl PublicBus {
    pub async fn new() -> Result<Self> {
        let mainloop = Mainloop::new().await?;
        let shutdown = mainloop.shutdown().clone();
        let join = tokio::spawn(mainloop.run());

        Ok(Self { shutdown, join })
    }
}

struct Mainloop {
    handle: BrokerHandle,
    join: JoinHandle<()>,
    listener: TcpListener,
    shutdown: Arc<Notify>,
}

impl Mainloop {
    async fn new() -> Result<Self> {
        log::info!("Listening on {BIND} for public connections.");

        let listener = TcpListener::bind(BIND)
            .await
            .with_context(|| anyhow!("failed to bind TCP listener to {BIND}"))?;

        let broker = Broker::new();
        let handle = broker.handle().clone();
        let join = tokio::spawn(broker.run());

        let shutdown = Arc::new(Notify::new());

        Ok(Self {
            handle,
            join,
            listener,
            shutdown,
        })
    }

    fn shutdown(&self) -> &Arc<Notify> {
        &self.shutdown
    }

    async fn run(self) -> Result<()> {
        loop {
            tokio::select! {
                () = self.shutdown.notified() => break,

                res = self.listener.accept() => {
                    let (stream, addr) =
                        res.with_context(|| anyhow!("failed to accept TCP connection"))?;

                    tokio::spawn(Self::new_connection(self.handle.clone(), stream, addr));
                }
            }
        }

        log::info!("Shutting down.");
        Ok(())
    }

    async fn new_connection(handle: BrokerHandle, stream: TcpStream, addr: SocketAddr) {
        log::info!("New connection from {addr}.");

        if let Err(e) = Self::new_connection_impl(handle, stream).await {
            log::error!("Failed to accept connection from {}: {}.", addr, e);
        }
    }

    async fn new_connection_impl(handle: BrokerHandle, stream: TcpStream) -> Result<()> {
        todo!()
    }
}
