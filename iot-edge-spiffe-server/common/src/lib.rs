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

use std::time::SystemTime;

#[derive(Clone, Copy, Debug, serde::Deserialize, serde::Serialize)]
pub enum KeyType {
    ECP256,
    RSA2048,
    RSA4096,
}

#[cfg(feature = "tests")]
pub const CONFIG_DEFAULT_PATH: &str = "../Config.toml";

#[must_use]
pub fn get_epoch_time() -> u64 {
    let now = SystemTime::now();
    let epoch = now
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("Epoch should succeed");
    epoch.as_secs()
}
