use crate::bus::Bus;
use crate::shutdown_notifier::ShutdownNotifier;
use crate::utils;
use aldrin::core::tokio::TokioTransport;
use aldrin::Handle as ClientHandle;
use aldrin_broker::BrokerHandle;
use anyhow::{anyhow, Context, Result};
use std::fs;
use std::ops::Deref;
use std::path::PathBuf;
use tokio::net::unix::UCred;
use tokio::net::{UnixListener, UnixStream};
use tokio::task::JoinHandle;

pub struct PrivateBus {
    client: ClientHandle,
    shutdown: ShutdownNotifier,
    join: JoinHandle<Result<()>>,
}

impl PrivateBus {
    pub async fn new() -> Result<Self> {
        let socket_path = utils::daemon_socket()?;
        log::info!(
            "Listening on `{}` for private connections.",
            socket_path.display()
        );

        let (shutdown, shutdown_mainloop) = ShutdownNotifier::new_pair();
        let mainloop = Mainloop::new(socket_path, shutdown_mainloop).await?;
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
            .with_context(|| anyhow!("failed to join private bus mainloop"))?
    }
}

impl Deref for PrivateBus {
    type Target = ClientHandle;

    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

struct Mainloop {
    shutdown: ShutdownNotifier,
    bus: Bus,
    listener: UnixListener,
    socket_path: PathBuf,
}

impl Mainloop {
    async fn new(socket_path: PathBuf, shutdown: ShutdownNotifier) -> Result<Self> {
        let _ = fs::remove_file(&socket_path);
        let listener = UnixListener::bind(&socket_path).with_context(|| {
            anyhow!(
                "failed to bind Unix listener to `{}`",
                socket_path.display()
            )
        })?;

        let bus = Bus::new().await?;

        Ok(Self {
            shutdown,
            bus,
            listener,
            socket_path,
        })
    }

    fn client(&self) -> &ClientHandle {
        self.bus.client()
    }

    async fn run(mut self) -> Result<()> {
        loop {
            tokio::select! {
                () = self.shutdown.wait() => break,
                () = self.bus.wait() => return Err(anyhow!("Private bus shut down unexpectedly")),

                res = self.listener.accept() => {
                    let (stream, _) =
                        res.with_context(|| anyhow!("failed to accept Unix connection"))?;

                    let broker = self.bus.broker().clone();
                    tokio::spawn(Self::new_connection(broker, stream));
                }
            }
        }

        log::info!("Shutting down.");
        let _ = fs::remove_file(self.socket_path);
        self.bus.shutdown().await?;

        Ok(())
    }

    async fn new_connection(handle: BrokerHandle, stream: UnixStream) {
        let pid = stream.peer_cred().ok().as_ref().and_then(UCred::pid);

        if let Some(pid) = pid {
            log::info!("New connection by process {pid}.");
        } else {
            log::info!("New connection.");
        }

        match (Self::new_connection_impl(handle, stream).await, pid) {
            (Ok(()), Some(pid)) => log::info!("Connection closed by process {pid}."),
            (Ok(()), None) => log::info!("Connection closed."),
            (Err(e), Some(pid)) => log::warn!("Connection by process {pid} failed: {e}."),
            (Err(e), None) => log::warn!("Connection failed: {e}."),
        }
    }

    async fn new_connection_impl(mut handle: BrokerHandle, stream: UnixStream) -> Result<()> {
        let transport = TokioTransport::new(stream);
        let conn = handle.connect(transport).await?;
        conn.run().await?;
        Ok(())
    }
}
