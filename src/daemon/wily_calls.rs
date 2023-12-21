use super::Mainloop;
use crate::schemas::{Metadata, Share, WilyFunction, WilyQueryArgs, WilyQueryError, WilyQueryOk};
use aldrin::Promise;
use anyhow::{anyhow, Result};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

impl Mainloop {
    pub(super) fn wily_call(&self, call: WilyFunction) {
        match call {
            WilyFunction::Query(args, promise) => self.wily_query(args, promise),
        }
    }

    fn wily_query(&self, args: WilyQueryArgs, promise: Promise<WilyQueryOk, WilyQueryError>) {
        log::info!("Querying path `{}`.", args.path);
        tokio::spawn(Self::wily_query_impl(args, promise, self.shares.clone()));
    }

    async fn wily_query_impl(
        args: WilyQueryArgs,
        promise: Promise<WilyQueryOk, WilyQueryError>,
        shares: Arc<RwLock<HashMap<String, Share>>>,
    ) -> Result<()> {
        let path = match Self::resolve_path(&shares, &args.path).await {
            Ok(path) => path,

            Err(e) => {
                log::error!("Failed to query `{}`: {}.", args.path, e);
                promise.err(&WilyQueryError::FileNotFound)?;
                return Ok(());
            }
        };

        match path {
            Some(path) => todo!(),

            None => {
                promise.ok(&WilyQueryOk::Root)?;
            }
        }

        Ok(())
    }

    async fn resolve_path(
        shares: &Arc<RwLock<HashMap<String, Share>>>,
        path: &str,
    ) -> Result<Option<PathBuf>> {
        let mut resolved = None;

        for component in path.split('/') {
            if component.is_empty() || (component == ".") {
                continue;
            }

            if let Some(ref mut resolved) = resolved {
                if component == ".." {
                    todo!()
                } else {
                    todo!()
                }
            } else {
                let shares = shares.read();
                let Some(share) = shares.get(component) else {
                    return Err(anyhow!("unknown share `{component}`"));
                };

                if share.disabled.any() {
                    return Err(anyhow!("share `{component}` is disabled"));
                }

                resolved = Some(PathBuf::from(&share.path));
            }
        }

        Ok(resolved)
    }
}
