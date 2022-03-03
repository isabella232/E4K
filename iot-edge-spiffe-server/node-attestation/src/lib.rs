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

pub mod psat;

#[cfg(not(any(test, feature = "tests")))]
use kube::Client;
#[cfg(any(test, feature = "tests"))]
use mock_kube::Client;

use std::{collections::HashMap, sync::Arc};

use catalog::NodeSelectorType;
use core_objects::{NodeSelector, SPIFFEID};
use server_config::NodeAttestationConfig;

#[derive(Clone, Debug)]
pub struct AgentAttributes {
    pub spiffe_id: SPIFFEID,
    pub selectors: HashMap<NodeSelectorType, NodeSelector>,
}

pub struct NodeAttestatorFactory {}

impl NodeAttestatorFactory {
    #[must_use]
    pub fn get(
        config: &NodeAttestationConfig,
        trust_domain: &str,
        client: Client,
    ) -> Arc<dyn NodeAttestation + Send + Sync> {
        match config {
            NodeAttestationConfig::Psat(config) => Arc::new(psat::NodeAttestation::new(
                config,
                trust_domain.to_string(),
                client,
            )),
            NodeAttestationConfig::Sat(_config) => unimplemented!(),
        }
    }
}

#[async_trait::async_trait]
pub trait NodeAttestation: Sync + Send {
    async fn attest_agent(
        &self,
        token: &str,
    ) -> Result<AgentAttributes, Box<dyn std::error::Error + Send>>;
}
