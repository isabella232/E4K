// Copyright (c) Microsoft. All rights reserved.

#![deny(rust_2018_idioms)]
#![warn(clippy::all, clippy::pedantic)]
#![allow(
    clippy::default_trait_access,
    clippy::let_and_return,
    clippy::let_unit_value,
    clippy::missing_errors_doc,
    clippy::similar_names,
    clippy::too_many_lines
)]

use std::error::Error as StdError;

use error::Error;
use server_config::Config;

const CONFIG_DEFAULT_PATH: &str = "../Config.toml";

mod error;

#[tokio::main]
async fn main() {
    logger::try_init()
        .expect("cannot fail to initialize global logger from the process entrypoint");

    log::info!("Starting IoTEdge SPIFFE Server");
    if let Err(err) = main_inner().await {
        log::error!("{}", err);

        let mut source = std::error::Error::source(&*err);
        while let Some(err) = source {
            log::error!("caused by: {}", err);
            source = std::error::Error::source(err);
        }

        std::process::exit(1);
    }
}

async fn main_inner() -> Result<(), Box<dyn StdError>> {
    let config = Config::load_config(CONFIG_DEFAULT_PATH).map_err(Error::ErrorParsingConfig)?;

    let catalog = catalog::load_catalog();

    admin_api::start_admin_api(&config, catalog).await?;
    server_api::start_server_api(&config).await?;

    Ok(())
}
