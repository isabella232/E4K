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

use common::KeyType;
use std::{fs, io, path::Path};

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct Config {
    pub socket_path: String,
    pub trust_domain: String,
    pub jwt_key_type: KeyType,
    pub jwt_key_ttl: u64,
    pub key_plugin_disk: Option<KeyPluginConfigDisk>,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct KeyPluginConfigDisk {
    pub key_base_path: String,
}

impl Config {
    pub fn load_config(filename: impl AsRef<Path>) -> Result<Config, io::Error> {
        let config = fs::read_to_string(&filename)?;

        let config = toml::from_str(&config)?;

        Ok(config)
    }
}
