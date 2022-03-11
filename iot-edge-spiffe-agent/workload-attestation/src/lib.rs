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

use agent_config::WorkloadAttestationConfig;
use core_objects::{WorkloadSelector, WorkloadSelectorType};

#[cfg(not(any(test, feature = "tests")))]
use kube::Client;
#[cfg(any(test, feature = "tests"))]
use mock_kube::Client;
#[cfg(feature = "tests")]
use mockall::automock;

use std::{collections::BTreeMap, sync::Arc};

#[derive(Clone, Debug, Default)]
pub struct WorkloadAttributes {
    pub selectors: BTreeMap<WorkloadSelectorType, WorkloadSelector>,
}

pub struct WorkloadAttestatorFactory {}

impl WorkloadAttestatorFactory {
    #[must_use]
    pub fn get(
        config: &WorkloadAttestationConfig,
        node_name: String,
        client: Client,
    ) -> Arc<dyn WorkloadAttestation + Send + Sync> {
        match config {
            WorkloadAttestationConfig::K8s(config) => {
                Arc::new(k8s::WorkloadAttestation::new(config, node_name, client))
            }
        }
    }
}

#[cfg_attr(feature = "tests", automock)]
#[async_trait::async_trait]
pub trait WorkloadAttestation: Sync + Send {
    async fn attest_workload(
        &self,
        pid: u32,
    ) -> Result<WorkloadAttributes, Box<dyn std::error::Error + Send>>;
}
