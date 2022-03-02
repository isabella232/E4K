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
use server_agent_api::{attest_agent, create_workload_jwt};

pub struct ServerClientFactory {}

impl ServerClientFactory {
    pub fn get(
        server_config: &ServerConfig,
    ) -> Result<Arc<dyn Client + Sync + Send>, Box<dyn std::error::Error + Send>> {
        let http_client = http::Client::new(server_config).map_err(|err| Box::new(err) as _)?;

        Ok(Arc::new(http_client))
    }
}

#[cfg_attr(feature = "tests", automock)]
#[async_trait::async_trait]
pub trait Client: Sync + Send {
    async fn create_workload_jwt(
        &self,
        request: create_workload_jwt::Request,
    ) -> Result<create_workload_jwt::Response, Box<dyn std::error::Error + Send>>;

    async fn attest_agent(
        &self,
        token: attest_agent::Auth,
    ) -> Result<attest_agent::Response, Box<dyn std::error::Error + Send>>;
}
