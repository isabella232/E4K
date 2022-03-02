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

use std::error::Error as StdError;

use agent_config::Config;
use error::Error;
use log::{error, info};
use node_attestation_agent::NodeAttestatorFactory;
use server_client::ServerClientFactory;

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
    let node_attestation =
        NodeAttestatorFactory::get(&config.node_attestation_config, server_api_client.clone());

    // Test code here
    let spiffe_id = node_attestation.attest_agent().await.unwrap();
    info!("Got spiffe id! {:?}", spiffe_id);

    Ok(())
}
