use tokio::task::{JoinError, JoinHandle};

#[derive(Debug)]
pub enum TaskHandle<T> {
    Pending(JoinHandle<T>),
    Ready(Result<T, JoinError>),
}

impl<T> TaskHandle<T> {
    pub fn new(inner: JoinHandle<T>) -> Self {
        Self::Pending(inner)
    }

    pub async fn wait(&mut self) {
        if let Self::Pending(inner) = self {
            let v = inner.await;
            *self = Self::Ready(v);
        }
    }

    pub async fn join(mut self) -> Result<T, JoinError> {
        self.wait().await;

        match self {
            Self::Pending(_) => unreachable!(),
            Self::Ready(v) => v,
        }
    }
}

impl<T> From<JoinHandle<T>> for TaskHandle<T> {
    fn from(inner: JoinHandle<T>) -> Self {
        Self::new(inner)
    }
}
