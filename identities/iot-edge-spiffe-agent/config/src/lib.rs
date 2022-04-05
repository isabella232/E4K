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

use std::{fs, io, path::Path};

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct Config {
    pub socket_path: String,
    pub trust_domain: String,

    #[serde(alias = "server-config")]
    pub server_config: ServerConfig,
    #[serde(
        alias = "trust-bundle-manager-config",
        default = "default_trust_bundle_manager_config"
    )]
    pub trust_bundle_config: TrustBundleManagerConfig,
    #[serde(
        alias = "node-attestation-config",
        default = "default_node_attestation_config"
    )]
    pub node_attestation_config: NodeAttestationConfig,
    #[serde(
        alias = "workload-attestation-config",
        default = "default_workload_attestation_config"
    )]
    pub workload_attestation_config: WorkloadAttestationConfig,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(tag = "type", content = "content", rename_all = "UPPERCASE")]
pub enum NodeAttestationConfig {
    Sat(NodeAttestationConfigK8s),
    Psat(NodeAttestationConfigK8s),
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct NodeAttestationConfigK8s {
    #[serde(default = "default_token_path")]
    pub token_path: String,
}

fn default_node_attestation_config() -> NodeAttestationConfig {
    let config = NodeAttestationConfigK8s {
        token_path: default_token_path(),
    };

    NodeAttestationConfig::Psat(config)
}

fn default_token_path() -> String {
    "/var/run/secrets/tokens/iotedge-spiffe-agent".to_string()
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(tag = "type", content = "content", rename_all = "UPPERCASE")]
pub enum WorkloadAttestationConfig {
    K8s(WorkloadAttestationConfigK8s),
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct WorkloadAttestationConfigK8s {
    #[serde(default = "default_max_poll_attempt")]
    pub max_poll_attempt: usize,
    #[serde(default = "default_poll_retry_interval_ms")]
    pub poll_retry_interval_ms: u64,
}

fn default_workload_attestation_config() -> WorkloadAttestationConfig {
    let config = WorkloadAttestationConfigK8s {
        max_poll_attempt: default_max_poll_attempt(),
        poll_retry_interval_ms: default_poll_retry_interval_ms(),
    };

    WorkloadAttestationConfig::K8s(config)
}

fn default_max_poll_attempt() -> usize {
    60
}

fn default_poll_retry_interval_ms() -> u64 {
    500
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct TrustBundleManagerConfig {
    #[serde(default = "default_max_retry")]
    pub max_retry: usize,
    #[serde(default = "default_wait_retry_sec")]
    pub wait_retry_sec: u64,
}

fn default_trust_bundle_manager_config() -> TrustBundleManagerConfig {
    TrustBundleManagerConfig {
        max_retry: default_max_retry(),
        wait_retry_sec: default_wait_retry_sec(),
    }
}

fn default_max_retry() -> usize {
    3
}

fn default_wait_retry_sec() -> u64 {
    2
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct ServerConfig {
    pub address: String,
    pub port: u16,
}

impl Config {
    pub fn load_config(filename: impl AsRef<Path>) -> Result<Config, io::Error> {
        let config = fs::read_to_string(&filename)?;

        let config = toml::from_str(&config)?;

        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use std::{fs::File, io::Read};

    use super::*;

    #[test]
    fn test_read_all() {
        let test_files_directory =
            std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/tests"));

        for test_file in std::fs::read_dir(test_files_directory).unwrap() {
            let test_file = test_file.unwrap();
            if test_file.file_type().unwrap().is_dir() {
                continue;
            }
            let test_file = test_file.path();

            println!("Parsing deployment file {:#?}", test_file);
            let mut raw_config = File::open(&test_file).unwrap();
            let mut buf = Vec::new();
            raw_config.read_to_end(&mut buf).unwrap();

            let _config: Config = toml::from_slice(&buf).unwrap();
        }
    }
}
