use super::Mainloop;
use crate::schemas::{
    DaemonDisableArgs, DaemonDisableError, DaemonEnableArgs, DaemonEnableError, DaemonFunction,
    DaemonShareArgs, DaemonShareError, DaemonUnshareArgs, DaemonUnshareError, DaemonUnsharedEvent,
    Share, ShareDisabled, ShareType, TransientShare, UnshareReason,
};
use aldrin::Promise;
use anyhow::{anyhow, Result};
use std::collections::hash_map::{Entry, HashMap};
use std::convert::Infallible;
use std::path::Path;

impl Mainloop {
    pub(super) async fn daemon_call(&mut self, call: DaemonFunction) -> Result<()> {
        match call {
            DaemonFunction::ShutDown(promise) => self.daemon_shut_down(promise),
            DaemonFunction::Share(args, promise) => self.daemon_share(args, promise).await,
            DaemonFunction::Unshare(args, promise) => self.daemon_unshare(args, promise).await,
            DaemonFunction::List(promise) => self.daemon_list(promise),
            DaemonFunction::Enable(args, promise) => self.daemon_enable(args, promise),
            DaemonFunction::Disable(args, promise) => self.daemon_disable(args, promise),
        }
    }

    fn daemon_shut_down(&mut self, promise: Promise<(), Infallible>) -> Result<()> {
        log::info!("Got a request to shut down.");
        self.shutdown = true;
        promise.done()?;
        Ok(())
    }

    async fn daemon_share(
        &mut self,
        args: DaemonShareArgs,
        promise: Promise<Share, DaemonShareError>,
    ) -> Result<()> {
        let name = match share_name(args.name.as_deref(), &args.path) {
            Ok(name) => name,

            Err(e) => {
                log::error!("Failed to add share: {e}.");
                promise.err(&DaemonShareError::InvalidName(e.to_string()))?;
                return Ok(());
            }
        };

        let mut shares = self.shares.write();

        let Entry::Vacant(entry) = shares.entry(name.to_owned()) else {
            log::error!("Duplicate share name `{name}`.");
            promise.err(&DaemonShareError::DuplicateName(name.to_owned()))?;
            return Ok(());
        };

        if Path::new(&args.path).is_relative() {
            log::error!("Cannot share relativ path `{}`.", args.path);
            promise.err(&DaemonShareError::RelativePath)?;
            return Ok(());
        }

        let share_type = if args.persist.unwrap_or(false) {
            todo!()
        } else {
            ShareType::Transient(TransientShare {
                expires_unix_ms: args.expires_unix_ms,
            })
        };

        if args.disabled.unwrap_or(false) {
            todo!()
        }

        log::info!("Sharing `{}` as `{}`.", args.path, name);

        let share = Share {
            name: name.to_owned(),
            path: args.path,
            share_type,
            disabled: ShareDisabled {
                user: args.disabled.unwrap_or(false),
            },
        };

        let share = entry.insert(share);
        self.daemon.shared(share)?;
        promise.ok(share)?;
        Ok(())
    }

    async fn daemon_unshare(
        &mut self,
        args: DaemonUnshareArgs,
        promise: Promise<Share, DaemonUnshareError>,
    ) -> Result<()> {
        let mut shares = self.shares.write();

        let entry = match shares.entry(args.name) {
            Entry::Occupied(entry) => entry,

            Entry::Vacant(entry) => {
                log::error!("Cannot remove unknown share `{}`", entry.key());
                promise.err(&DaemonUnshareError::UnknownShare)?;
                return Ok(());
            }
        };

        match entry.get().share_type {
            ShareType::Static => {
                log::error!("Cannot remove static share `{}`", entry.key());
                promise.err(&DaemonUnshareError::StaticShare)?;
                return Ok(());
            }

            ShareType::Persisted(_) => todo!(),
            ShareType::Transient(_) => {}
        }

        let share = entry.remove();
        log::info!("Removing share `{}` (`{}`).", share.name, share.path);
        promise.ok(&share)?;

        self.daemon.unshared(&DaemonUnsharedEvent {
            share,
            reason: UnshareReason::UserRequest,
        })?;

        Ok(())
    }

    fn daemon_list(&self, promise: Promise<HashMap<String, Share>, Infallible>) -> Result<()> {
        log::info!("Listing all shares.");
        promise.ok(&self.shares.read())?;
        Ok(())
    }

    fn daemon_enable(
        &self,
        _args: DaemonEnableArgs,
        _promise: Promise<Share, DaemonEnableError>,
    ) -> Result<()> {
        todo!()
    }

    fn daemon_disable(
        &self,
        _args: DaemonDisableArgs,
        _promise: Promise<Share, DaemonDisableError>,
    ) -> Result<()> {
        todo!()
    }
}

fn share_name<'a>(name: Option<&'a str>, path: &'a str) -> Result<&'a str> {
    if let Some(name) = name {
        if name.is_empty() {
            Err(anyhow!("share name is empty"))
        } else if name.contains('\0') {
            Err(anyhow!("share name contains a NUL character"))
        } else if name.contains('/') {
            Err(anyhow!("share name contains a `/` separator"))
        } else {
            Ok(name)
        }
    } else if let Some(file_name) = Path::new(path).file_name() {
        Ok(file_name.to_str().unwrap())
    } else {
        Err(anyhow!("failed to derive share name for path `{path}`"))
    }
}
