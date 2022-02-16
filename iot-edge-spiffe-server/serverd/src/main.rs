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

use catalog::{Catalog, CatalogFactory};
use config::Config;
use core_objects::get_epoch_time;
use error::Error;
use futures_util::{future, pin_mut};
use key_manager::KeyManager;
use key_store::KeyStoreFactory;
use log::{error, info};
use std::{error::Error as StdError, sync::Arc, time::Duration};
use svid_factory::SVIDFactory;
use tokio::{sync::Notify, time};
use trust_bundle_builder::TrustBundleBuilder;

const CONFIG_DEFAULT_PATH: &str = "../Config.toml";

const KEY_MANAGER_ROTATION_POLL_INTERVAL_SECONDS: u64 = 10;

mod error;

#[tokio::main]
async fn main() {
    logger::try_init()
        .expect("cannot fail to initialize global logger from the process entrypoint");

    info!("Starting IoTEdge SPIFFE Server");
    if let Err(err) = main_inner().await {
        error!("{}", err);

        let mut source = std::error::Error::source(&*err);
        while let Some(err) = source {
            error!("caused by: {}", err);
            source = std::error::Error::source(err);
        }

        std::process::exit(1);
    }
}

async fn main_inner() -> Result<(), Box<dyn StdError>> {
    let config = Config::load_config(CONFIG_DEFAULT_PATH).map_err(Error::ErrorParsingConfig)?;

    let catalog: Arc<dyn Catalog + Send + Sync> = CatalogFactory::get(&config.catalog);

    let key_store = KeyStoreFactory::get(&config.key_store);

    let key_manager =
        KeyManager::new(&config, catalog.clone(), key_store, get_epoch_time()).await?;
    let key_manager = Arc::new(key_manager);

    let svid_factory = SVIDFactory::new(key_manager.clone(), &config);
    let svid_factory = Arc::new(svid_factory);

    let trust_bundle_builder = TrustBundleBuilder::new(&config, catalog.clone());

    let key_manager_shutdown_signal_rx = Arc::new(Notify::new());
    let key_manager_shutdown_signal_tx = key_manager_shutdown_signal_rx.clone();
    let key_manager_handle = tokio::spawn(async move {
        info!("Starting Key manager");
        let mut interval = time::interval(Duration::from_secs(
            KEY_MANAGER_ROTATION_POLL_INTERVAL_SECONDS,
        ));

        loop {
            let wait_shutdown = key_manager_shutdown_signal_rx.notified();
            let wait_tick = interval.tick();

            pin_mut!(wait_shutdown);
            pin_mut!(wait_tick);

            match future::select(wait_shutdown, wait_tick).await {
                future::Either::Left(_) => {
                    info!("Closing key manager task");
                    break;
                }
                future::Either::Right(_) => {
                    if let Err(err) = key_manager.rotate_periodic().await {
                        error!("{}", err);
                    }
                }
            };
        }
    });

    let admin_api_handle = admin_api::start_admin_api(&config, catalog.clone()).await?;
    let server_api_handle =
        server_api::start_server_api(&config, catalog, svid_factory, trust_bundle_builder).await?;

    let _wait = admin_api_handle.await;
    let _wait = server_api_handle.await;

    key_manager_shutdown_signal_tx.notify_one();
    let _wait = key_manager_handle.await;

    Ok(())
}
