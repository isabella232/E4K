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
#[cfg(feature = "tests")]
use mockall::automock;

pub struct NodeAttestatorFactory {}

impl NodeAttestatorFactory {
    #[must_use]
    pub fn get(config: &NodeAttestationConfig) -> Arc<dyn NodeAttestation> {
        match config {
            NodeAttestationConfig::Sat(config) | NodeAttestationConfig::Psat(config) => {
                Arc::new(k8s::NodeAttestation::new(config))
            }
        }
    }
}

#[cfg_attr(feature = "tests", automock)]
#[async_trait::async_trait]
pub trait NodeAttestation: Sync + Send {
    async fn get_attestation_token(&self) -> Result<String, Box<dyn std::error::Error + Send>>;
}
