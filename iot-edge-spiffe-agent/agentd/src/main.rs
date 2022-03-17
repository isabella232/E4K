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

mod error;
use agent_config::Config;
use error::Error;
use futures_util::{future, pin_mut, TryFutureExt};
use jwt_svid_validator::validate;
#[cfg(not(any(test, feature = "tests")))]
use kube::Client;
use log::{error, info};
#[cfg(any(test, feature = "tests"))]
use mock_kube::Client;
use node_attestation_agent::NodeAttestatorFactory;
use spiffe_server_client::ServerClientFactory;
use std::{env, error::Error as StdError, sync::Arc, time::Duration};
use tokio::{fs, net::UnixListener, sync::Notify, task::JoinHandle, time};
use tonic::transport::Server;
use trust_bundle_manager::TrustBundleManager;
use workload_api::spiffe_workload_api_server::SpiffeWorkloadApiServer;
use workload_api_server::{unix_stream, WorkloadAPIServer};
use workload_attestation::WorkloadAttestatorFactory;

const CONFIG_DEFAULT_PATH: &str = "/mnt/config/Config.toml";
const NODE_NAME_ENV_VAR: &str = "NODE_NAME";

#[tokio::main]
async fn main() {
    logger::try_init()
        .expect("cannot fail to initialize global logger from the process entrypoint");

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
    info!("Starting IoTEdge SPIFFE Agent");

    let config = Config::load_config(CONFIG_DEFAULT_PATH).map_err(Error::ParsingConfig)?;

    let node_name = env::var(NODE_NAME_ENV_VAR)?;

    let kube_client = Client::try_default().await?;

    let server_api_client =
        ServerClientFactory::get(&config.server_config).map_err(Error::CreatingServerclient)?;

    let node_attestation = NodeAttestatorFactory::get(&config.node_attestation_config);

    let workload_attestation =
        WorkloadAttestatorFactory::get(&config.workload_attestation_config, node_name, kube_client);

    let trust_bundle = TrustBundleManager::get_init_trust_bundle(
        server_api_client.clone(),
        &config.trust_bundle_config,
    )
    .await?;
    let jwt_trust_bundle_refresh_hint = trust_bundle.jwt_key_set.spiffe_refresh_hint;
    let trust_bundle_manager = Arc::new(TrustBundleManager::new(
        server_api_client.clone(),
        trust_bundle,
    ));
    let (trust_bundle_manager_handle, trust_bundle_manager_shutdown_signal_tx) =
        start_refresh_trust_bundle_task(
            trust_bundle_manager.clone(),
            jwt_trust_bundle_refresh_hint,
        )
        .await;

    let jwt_svid_validator = Arc::new(validate::JWTSVIDValidator::default());

    let uds_stream = {
        let _result = fs::remove_file(config.socket_path.clone()).await;
        let uds = UnixListener::bind(config.socket_path)?;

        async_stream::stream! {
            loop {
                let item = uds.accept().map_ok(|(st, _)| unix_stream::UnixStream(st)).await;

                yield item;
            }
        }
    };

    info!("Starting workload API server");

    Server::builder()
        .add_service(SpiffeWorkloadApiServer::new(WorkloadAPIServer::new(
            server_api_client,
            workload_attestation,
            node_attestation,
            trust_bundle_manager,
            jwt_svid_validator,
        )))
        .serve_with_incoming(uds_stream)
        .await?;

    trust_bundle_manager_shutdown_signal_tx.notify_one();
    let _wait = trust_bundle_manager_handle.await;

    Ok(())
}

async fn start_refresh_trust_bundle_task(
    trust_bundle_manager: Arc<TrustBundleManager>,
    refresh_period_sec: u64,
) -> (JoinHandle<()>, Arc<Notify>) {
    let trust_bundle_manager_shutdown_signal_rx = Arc::new(Notify::new());
    let trust_bundle_manager_shutdown_signal_tx = trust_bundle_manager_shutdown_signal_rx.clone();
    let trust_bundle_manager_handle = tokio::spawn(async move {
        info!("Starting Trust Bundle manager refresh task");
        let mut interval = time::interval(Duration::from_secs(refresh_period_sec));

        loop {
            let wait_shutdown = trust_bundle_manager_shutdown_signal_rx.notified();
            let wait_tick = interval.tick();

            pin_mut!(wait_shutdown);
            pin_mut!(wait_tick);

            match future::select(wait_shutdown, wait_tick).await {
                future::Either::Left(_) => {
                    info!("Closing key manager task");
                    break;
                }
                future::Either::Right(_) => {
                    if let Err(err) = trust_bundle_manager.refresh_trust_bundle().await {
                        error!("{}", err);
                    } else {
                        info!("Fetch new trust bundle");
                    }
                }
            };
        }
    });

    (
        trust_bundle_manager_handle,
        trust_bundle_manager_shutdown_signal_tx,
    )
}
