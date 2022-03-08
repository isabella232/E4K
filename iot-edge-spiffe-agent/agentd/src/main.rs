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
use log::{error, info};
use node_attestation_agent::NodeAttestatorFactory;
use spiffe_server_client::ServerClientFactory;
use std::error::Error as StdError;
use tokio::{fs, net::UnixListener};
use tonic::transport::Server;
use workload_api::spiffe_workload_api_server::SpiffeWorkloadApiServer;
use workload_api_server::{unix_stream, WorkloadAPIServer};

const CONFIG_DEFAULT_PATH: &str = "/mnt/config/Config.toml";

#[tokio::main]
async fn main() {
    logger::try_init()
        .expect("cannot fail to initialize global logger from the process entrypoint");

    info!("Starting IoTEdge SPIFFE Agent");
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
    let config = Config::load_config(CONFIG_DEFAULT_PATH).map_err(Error::ParsingConfig)?;

    let server_api_client =
        ServerClientFactory::get(&config.server_config).map_err(Error::CreatingServerclient)?;
    let _node_attestation =
        NodeAttestatorFactory::get(&config.node_attestation_config, server_api_client.clone());

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
        )))
        .serve_with_incoming(uds_stream)
        .await?;

    Ok(())
}
