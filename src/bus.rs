use crate::shutdown_notifier::ShutdownNotifier;
use crate::task_handle::TaskHandle;
use aldrin::core::channel;
use aldrin::{Client, Handle as ClientHandle};
use aldrin_broker::{Broker, BrokerHandle};
use anyhow::{anyhow, Context, Error, Result};
use futures::TryFutureExt;
use tokio::task::JoinHandle;

#[derive(Debug)]
pub struct Bus {
    shutdown: ShutdownNotifier,
    join: JoinHandle<Result<()>>,
    broker: BrokerHandle,
    client: ClientHandle,
}

impl Bus {
    pub async fn new() -> Result<Self> {
        log::debug!("Creating a bus.");

        let (shutdown, shutdown_mainloop) = ShutdownNotifier::new_pair();
        let mainloop = Mainloop::new(shutdown_mainloop).await?;
        let broker = mainloop.broker().clone();
        let client = mainloop.client().clone();
        let join = tokio::spawn(mainloop.run());

        Ok(Self {
            shutdown,
            join,
            broker,
            client,
        })
    }

    pub fn broker(&self) -> &BrokerHandle {
        &self.broker
    }

    pub fn client(&self) -> &ClientHandle {
        &self.client
    }

    pub async fn wait(&mut self) {
        self.shutdown.wait().await
    }

    pub async fn shutdown(mut self) -> Result<()> {
        self.shutdown.shutdown();

        self.join
            .await
            .with_context(|| anyhow!("failed to join mainloop"))?
    }
}

#[derive(Debug)]
struct Mainloop {
    shutdown: ShutdownNotifier,
    broker_handle: BrokerHandle,
    broker_task: TaskHandle<()>,
    client_handle: ClientHandle,
    client_task: TaskHandle<Result<()>>,
    conn_task: TaskHandle<Result<()>>,
}

impl Mainloop {
    async fn new(shutdown: ShutdownNotifier) -> Result<Self> {
        log::debug!("Creating broker.");
        let broker = Broker::new();
        let mut broker_handle = broker.handle().clone();
        let broker_task = tokio::spawn(broker.run()).into();

        log::debug!("Connecting internal client.");
        let (channel1, channel2) = channel::bounded(1);
        let (client, conn) =
            tokio::join!(Client::connect(channel1), broker_handle.connect(channel2));
        let client = client.with_context(|| anyhow!("failed to connect internal client"))?;
        let conn = conn.with_context(|| anyhow!("failed to connect internal client"))?;
        let client_handle = client.handle().clone();

        let client_task = tokio::spawn(client.run().map_err(Error::from)).into();
        let conn_task = tokio::spawn(conn.run().map_err(Error::from)).into();

        Ok(Self {
            shutdown,
            broker_handle,
            broker_task,
            client_handle,
            client_task,
            conn_task,
        })
    }

    fn broker(&self) -> &BrokerHandle {
        &self.broker_handle
    }

    fn client(&self) -> &ClientHandle {
        &self.client_handle
    }

    async fn run(mut self) -> Result<()> {
        log::debug!("Entering mainloop.");

        tokio::select! {
            () = self.shutdown.wait() => {}
            () = self.broker_task.wait() => log::error!("Broker shut down unexpectedly."),
            () = self.client_task.wait() => log::error!("Internal client shut down unexpectedly."),

            () = self.conn_task.wait() => {
                log::error!("Internal client connection shut down unexpectedly.");
            }
        }

        log::debug!("Shutting down mainloop.");

        self.broker_handle.shutdown().await;
        self.client_handle.shutdown();

        self.broker_task
            .join()
            .await
            .with_context(|| anyhow!("failed to join broker"))?;

        self.client_task
            .join()
            .await
            .with_context(|| anyhow!("failed to join internal client"))??;

        self.conn_task
            .join()
            .await
            .with_context(|| anyhow!("failed to join internal client connection"))??;

        Ok(())
    }
}
