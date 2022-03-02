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

pub mod k8s;

use std::sync::Arc;

use agent_config::NodeAttestationConfig;
use core_objects::JWTSVIDCompact;
use server_client::Client;

pub struct NodeAttestatorFactory {}

impl NodeAttestatorFactory {
    #[must_use]
    pub fn get(
        config: &NodeAttestationConfig,
        server_api_client: Arc<dyn Client + Sync + Send>,
    ) -> Arc<dyn NodeAttestation + Send + Sync> {
        match config {
            NodeAttestationConfig::Sat(config) | NodeAttestationConfig::Psat(config) => {
                Arc::new(k8s::NodeAttestation::new(config, server_api_client))
            }
        }
    }
}

#[async_trait::async_trait]
pub trait NodeAttestation: Sync + Send {
    async fn attest_agent(&self) -> Result<JWTSVIDCompact, Box<dyn std::error::Error + Send>>;
}
