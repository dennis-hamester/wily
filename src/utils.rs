use crate::schemas::{
    DaemonProxy, Share, ShareType, WilyProxy, DAEMON_OBJECT_UUID, DAEMON_UUID, WILY_OBJECT_UUID,
    WILY_UUID,
};
use aldrin::core::tokio::TokioTransport;
use aldrin::Client;
use anyhow::{anyhow, Context, Error, Result};
use chrono::{Local, TimeZone};
use futures::TryFutureExt;
use std::borrow::Cow;
use std::env;
use std::path::{Path, PathBuf};
use tokio::net::{TcpStream, UnixStream};
use tokio::task::JoinHandle;
use url::Url;

const WILY_PORT: u16 = 9999;

pub fn daemon_socket() -> Result<PathBuf> {
    let mut dir = dirs::runtime_dir()
        .or_else(dirs::data_local_dir)
        .ok_or_else(|| anyhow!("no directory available for the daemon's socket"))?;

    dir.push("wily.sock");
    Ok(dir)
}

pub async fn connect_daemon() -> Result<(DaemonProxy, JoinHandle<Result<()>>)> {
    let socket_path = daemon_socket()?;

    let transport = UnixStream::connect(&socket_path)
        .await
        .map(TokioTransport::new)
        .with_context(|| anyhow!("failed to connect to daemon at `{}`", socket_path.display()))?;

    let client = Client::connect(transport)
        .await
        .with_context(|| anyhow!("failed to connect to daemon at `{}`", socket_path.display()))?;

    let handle = client.handle().clone();
    let join = tokio::spawn(client.run().map_err(Error::from));

    let (_, [id]) = handle
        .find_specific_object(DAEMON_OBJECT_UUID, &[DAEMON_UUID])
        .await?
        .ok_or_else(|| anyhow!("Wily daemon not found at `{}`", socket_path.display()))?;

    let daemon = DaemonProxy::new(handle, id).await?;
    Ok((daemon, join))
}

pub async fn connect_wily(url: &Url) -> Result<(WilyProxy, JoinHandle<Result<()>>)> {
    let (host, port) = verify_url(url)?;

    let transport = TcpStream::connect((host, port))
        .await
        .map(TokioTransport::new)
        .with_context(|| anyhow!("failed to connect to `{host}:{port}`"))?;

    let client = Client::connect(transport)
        .await
        .with_context(|| anyhow!("failed to connect to `{host}:{port}`"))?;

    let handle = client.handle().clone();
    let join = tokio::spawn(client.run().map_err(Error::from));

    let (_, [id]) = handle
        .find_specific_object(WILY_OBJECT_UUID, &[WILY_UUID])
        .await?
        .ok_or_else(|| anyhow!("Wily not found at `{host}:{port}`"))?;

    let wily = WilyProxy::new(handle, id).await?;
    Ok((wily, join))
}

fn verify_url(url: &Url) -> Result<(&str, u16)> {
    if url.scheme() != "wily" {
        return Err(anyhow!("invalid URL scheme `{}`", url.scheme()));
    }

    let host = url
        .host_str()
        .ok_or_else(|| anyhow!("URL `{url}` doesn't have a host"))?;

    let port = url.port_or_known_default().unwrap_or(WILY_PORT);

    Ok((host, port))
}

pub fn ensure_absolute(path: &Path) -> Result<Cow<Path>> {
    if path.is_absolute() {
        Ok(Cow::Borrowed(path))
    } else {
        let mut absolute =
            env::current_dir().with_context(|| anyhow!("failed to determine current directory"))?;

        absolute.push(path);
        Ok(Cow::Owned(absolute))
    }
}

pub fn print_share(share: &Share) {
    fn print_expires(ts_unix_ms: Option<i64>) {
        print!("Expires:  ");

        if let Some(ts_unix_ms) = ts_unix_ms {
            let expires = Local
                .timestamp_millis_opt(ts_unix_ms)
                .single()
                .unwrap()
                .naive_local();

            println!("{}", expires);
        } else {
            println!("never");
        }
    }

    println!("Name:     {}", share.name);
    println!("Path:     {}", share.path);

    print!("Type:     ");
    match share.share_type {
        ShareType::Static => println!("static"),

        ShareType::Persisted(ref persisted) => {
            println!("persisted");
            print_expires(persisted.expires_unix_ms);
        }

        ShareType::Transient(ref transient) => {
            println!("transient");
            print_expires(transient.expires_unix_ms);
        }
    }

    print!("Disabled: ");
    if share.disabled.any() {
        if share.disabled.user {
            print!("user");
        }

        println!();
    } else {
        println!("no");
    }
}
