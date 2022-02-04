// Copyright (c) Microsoft. All rights reserved.

#![deny(rust_2018_idioms)]
#![warn(clippy::all, clippy::pedantic)]
#![allow(
    clippy::missing_safety_doc,
    clippy::default_trait_access,
    clippy::let_unit_value,
    clippy::missing_errors_doc,
    clippy::similar_names,
    clippy::too_many_lines
)]

use common::KeyType;
use openssl::pkey::{PKey, Public};

pub mod disk;

#[async_trait::async_trait]
pub trait KeyPlugin: Sync + Send {
    type Error: std::error::Error + 'static;

    async fn create_key_pair_if_not_exists(
        &self,
        id: &str,
        key_type: KeyType,
    ) -> Result<(), Self::Error>;
    async fn sign(
        &self,
        id: &str,
        key_type: KeyType,
        digest: &[u8],
    ) -> Result<(usize, Vec<u8>), Self::Error>;
    async fn delete_key_pair(&self, id: &str) -> Result<(), Self::Error>;
    async fn get_public_key(&self, id: &str) -> Result<PKey<Public>, Self::Error>;
}
