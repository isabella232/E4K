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

use core_objects::KeyType;

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct Config {
    pub socket_path: String,
    #[serde(alias = "server-agent-api")]
    pub server_agent_api: ServerAgentAPI,
    pub trust_domain: String,
    pub jwt: JWTConfig,
    #[serde(alias = "trust-bundle")]
    pub trust_bundle: TrustBundleConfig,
    #[serde(alias = "key-store")]
    pub key_store: KeyStoreConfig,
    pub catalog: CatalogConfig,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct ServerAgentAPI {
    pub bind_address: String,
    pub bind_port: u16,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct JWTConfig {
    pub key_type: KeyType,
    pub key_ttl: u64,
    pub ttl: u64,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct TrustBundleConfig {
    pub refresh_hint: u64,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(tag = "type", content = "args")]
pub enum KeyStoreConfig {
    Disk(KeyStoreConfigDisk),
    Memory(),
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(tag = "type")]
pub enum CatalogConfig {
    Disk,
    Memory,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct KeyStoreConfigDisk {
    pub key_base_path: String,
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
