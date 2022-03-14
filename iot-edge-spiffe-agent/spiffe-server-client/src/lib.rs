// Copyright (c) Microsoft. All rights reserved.

#![deny(rust_2018_idioms)]
#![warn(clippy::all, clippy::pedantic)]
#![allow(
    clippy::default_trait_access,
    clippy::let_unit_value,
    clippy::missing_errors_doc,
    clippy::similar_names,
    clippy::too_many_lines,
    clippy::missing_panics_doc
)]

pub mod http;

use std::sync::Arc;

#[cfg(feature = "tests")]
use mockall::automock;

use agent_config::ServerConfig;
use server_agent_api::{create_workload_jwts, get_trust_bundle};

pub struct ServerClientFactory {}

impl ServerClientFactory {
    pub fn get(
        server_config: &ServerConfig,
    ) -> Result<Arc<dyn Client>, Box<dyn std::error::Error + Send>> {
        let http_client = http::Client::new(server_config).map_err(|err| Box::new(err) as _)?;

        Ok(Arc::new(http_client))
    }
}

#[cfg_attr(feature = "tests", automock)]
#[async_trait::async_trait]
pub trait Client: Sync + Send {
    async fn create_workload_jwts(
        &self,
        request: create_workload_jwts::Request,
    ) -> Result<create_workload_jwts::Response, Box<dyn std::error::Error + Send>>;

    async fn get_trust_bundle(
        &self,
        params: get_trust_bundle::Params,
    ) -> Result<get_trust_bundle::Response, Box<dyn std::error::Error + Send>>;
}
