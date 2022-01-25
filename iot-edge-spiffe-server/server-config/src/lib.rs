// Copyright (c) Microsoft. All rights reserved.

#![deny(rust_2018_idioms)]
#![warn(clippy::all, clippy::pedantic)]
#![allow(
    clippy::default_trait_access,
    clippy::let_and_return,
    clippy::let_unit_value,
    clippy::missing_errors_doc,
    clippy::similar_names,
    clippy::too_many_lines
)]

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct Config {
    pub socket_path: String,
}

impl Config {
    pub fn load_config(filename: impl AsRef<std::path::Path>) -> Result<Config, std::io::Error> {
        let config = std::fs::read_to_string(&filename)?;

        let config = toml::from_str(&config)?;

        Ok(config)
    }
}
