// Copyright (c) Microsoft. All rights reserved.

#![deny(rust_2018_idioms)]
#![warn(clippy::all, clippy::pedantic)]
#![allow(
    clippy::missing_safety_doc,
    clippy::default_trait_access,
    clippy::let_unit_value,
    clippy::missing_errors_doc,
    clippy::similar_names,
    clippy::too_many_lines,
    clippy::missing_panics_doc
)]

use std::sync::Arc;

use core_objects::KeyType;
use openssl::pkey::{PKey, Public};
use server_config::KeyStoreConfig;

pub mod disk;

pub struct KeyStoreFactory {}

impl KeyStoreFactory {
    #[must_use]
    pub fn get(config: &KeyStoreConfig) -> Arc<dyn KeyStore> {
        match config {
            KeyStoreConfig::Disk(config) => Arc::new(disk::KeyStore::new(config)),
            KeyStoreConfig::Memory() => unimplemented!(),
        }
    }
}

#[async_trait::async_trait]
pub trait KeyStore: Sync + Send {
    async fn create_key_pair_if_not_exists(
        &self,
        id: &str,
        key_type: KeyType,
    ) -> Result<PKey<Public>, Box<dyn std::error::Error + Send>>;
    async fn sign(
        &self,
        id: &str,
        key_type: KeyType,
        digest: &[u8],
    ) -> Result<(usize, Vec<u8>), Box<dyn std::error::Error + Send>>;
    async fn delete_key_pair(&self, id: &str) -> Result<(), Box<dyn std::error::Error + Send>>;
    async fn get_public_key(
        &self,
        id: &str,
    ) -> Result<PKey<Public>, Box<dyn std::error::Error + Send>>;
}
