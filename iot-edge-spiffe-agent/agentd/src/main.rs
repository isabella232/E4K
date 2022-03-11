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
use futures_util::TryFutureExt;
#[cfg(not(any(test, feature = "tests")))]
use kube::Client;
use log::{error, info};
#[cfg(any(test, feature = "tests"))]
use mock_kube::Client;
use node_attestation_agent::NodeAttestatorFactory;
use spiffe_server_client::ServerClientFactory;
use std::{env, error::Error as StdError};
use tokio::{fs, net::UnixListener};
use tonic::transport::Server;
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
    let _node_attestation =
        NodeAttestatorFactory::get(&config.node_attestation_config, server_api_client.clone());
    let workload_attestation =
        WorkloadAttestatorFactory::get(&config.workload_attestation_config, node_name, kube_client);

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
        )))
        .serve_with_incoming(uds_stream)
        .await?;

    Ok(())
}
