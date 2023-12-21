use crate::bus::Bus;
use crate::shutdown_notifier::ShutdownNotifier;
use aldrin::core::tokio::TokioTransport;
use aldrin::Handle as ClientHandle;
use aldrin_broker::BrokerHandle;
use anyhow::{anyhow, Context, Result};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::ops::Deref;
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinHandle;

const BIND: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 9999);

pub struct PublicBus {
    client: ClientHandle,
    shutdown: ShutdownNotifier,
    join: JoinHandle<Result<()>>,
}

impl PublicBus {
    pub async fn new() -> Result<Self> {
        log::info!("Listening on {BIND} for public connections.");

        let (shutdown, shutdown_mainloop) = ShutdownNotifier::new_pair();
        let mainloop = Mainloop::new(shutdown_mainloop).await?;
        let client = mainloop.client().clone();
        let join = tokio::spawn(mainloop.run());

        Ok(Self {
            client,
            shutdown,
            join,
        })
    }

    pub async fn wait(&mut self) {
        self.shutdown.wait().await
    }

    pub async fn shutdown(mut self) -> Result<()> {
        self.shutdown.shutdown();

        self.join
            .await
            .with_context(|| anyhow!("failed to join public bus mainloop"))?
    }
}

impl Deref for PublicBus {
    type Target = ClientHandle;

    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

struct Mainloop {
    shutdown: ShutdownNotifier,
    bus: Bus,
    listener: TcpListener,
}

impl Mainloop {
    async fn new(shutdown: ShutdownNotifier) -> Result<Self> {
        let listener = TcpListener::bind(BIND)
            .await
            .with_context(|| anyhow!("failed to bind TCP listener to {BIND}"))?;

        let bus = Bus::new().await?;

        Ok(Self {
            shutdown,
            bus,
            listener,
        })
    }

    fn client(&self) -> &ClientHandle {
        self.bus.client()
    }

    async fn run(mut self) -> Result<()> {
        loop {
            tokio::select! {
                () = self.shutdown.wait() => break,
                () = self.bus.wait() => return Err(anyhow!("Public bus shut down unexpectedly")),

                res = self.listener.accept() => {
                    let (stream, addr) =
                        res.with_context(|| anyhow!("failed to accept TCP connection"))?;

                    let broker = self.bus.broker().clone();
                    tokio::spawn(Self::new_connection(broker, stream, addr));
                }
            }
        }

        log::info!("Shutting down.");
        self.bus.shutdown().await?;

        Ok(())
    }

    async fn new_connection(handle: BrokerHandle, stream: TcpStream, addr: SocketAddr) {
        log::info!("New connection from {addr}.");

        match Self::new_connection_impl(handle, stream).await {
            Ok(()) => log::info!("Connection closed by peer {addr}."),
            Err(e) => log::error!("Connection by peer {addr} failed: {e}."),
        }
    }

    async fn new_connection_impl(mut handle: BrokerHandle, stream: TcpStream) -> Result<()> {
        let transport = TokioTransport::new(stream);
        let conn = handle.connect(transport).await?;
        conn.run().await?;
        Ok(())
    }
}
