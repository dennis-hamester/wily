use tokio::sync::oneshot::{self, Receiver, Sender};

#[derive(Debug)]
pub struct ShutdownNotifier {
    inner: Option<Inner>,
}

impl ShutdownNotifier {
    pub fn new_pair() -> (Self, Self) {
        let (sender, receiver) = oneshot::channel();

        (
            Self {
                inner: Some(Inner::Sender(sender)),
            },
            Self {
                inner: Some(Inner::Receiver(receiver)),
            },
        )
    }

    pub fn shutdown(&mut self) {
        self.inner = None;
    }

    pub async fn wait(&mut self) {
        if let Some(inner) = self.inner.as_mut() {
            inner.wait().await;
            self.inner = None;
        }
    }
}

#[derive(Debug)]
enum Inner {
    Sender(Sender<()>),
    Receiver(Receiver<()>),
}

impl Inner {
    async fn wait(&mut self) {
        match self {
            Self::Sender(sender) => sender.closed().await,

            Self::Receiver(receiver) => {
                let _ = receiver.await;
            }
        }
    }
}
