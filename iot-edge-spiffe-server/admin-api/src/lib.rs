// Copyright (c) Microsoft. All rights reserved.

#![deny(rust_2018_idioms)]
#![warn(clippy::all, clippy::pedantic)]
#![allow(
    clippy::default_trait_access,
    clippy::let_unit_value,
    clippy::missing_errors_doc,
    clippy::similar_names,
    clippy::too_many_lines
)]

use catalog::Catalog;
use http_common::Connector;
use server_config::Config;
use std::{io, path::Path, sync::Arc};
use tokio::task::JoinHandle;

pub mod entries_api;
mod error;
mod http;

const SOCKET_DEFAULT_PERMISSION: u32 = 0o660;

pub async fn start_admin_api(
    config: &Config,
    catalog: Arc<dyn Catalog>,
) -> Result<JoinHandle<Result<(), std::io::Error>>, io::Error> {
    let api = Api { catalog };

    let service = http::Service { api: api.clone() };

    let connector = Connector::Unix {
        socket_path: Path::new(&config.socket_path).into(),
    };

    let mut incoming = connector.incoming(SOCKET_DEFAULT_PERMISSION, None).await?;

    Ok(tokio::spawn(async move {
        // Channel to gracefully shut down the server. It's currently not used.
        let (_shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();

        log::info!("Starting admin server");
        let res = incoming.serve(service, shutdown_rx).await;
        if let Err(err) = res {
            log::error!("Closing admin server: {:?}", err);
        } else {
            log::info!("Closing admin server");
        };

        Ok(())
    }))
}

#[derive(Clone)]
struct Api {
    catalog: Arc<dyn Catalog>,
}
