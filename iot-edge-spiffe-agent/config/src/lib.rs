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
    #[serde(alias = "trust-bundle-config")]
    pub trust_bundle_config: TrustBundleConfig,
    #[serde(alias = "node-attestation-config")]
    pub node_attestation_config: NodeAttestationConfig,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(tag = "type", content = "content", rename_all = "UPPERCASE")]
pub enum NodeAttestationConfig {
    Sat(NodeAttestationConfigK8s),
    Psat(NodeAttestationConfigK8s),
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct NodeAttestationConfigK8s {
    pub token_path: String,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(tag = "type", content = "content", rename_all = "UPPERCASE")]
pub enum TrustBundleConfig {
    Path(String),
    Url(String),
    InsecureBootstrap,
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
