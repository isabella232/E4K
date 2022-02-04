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

use openssl::pkey::{PKey, Public};
use server_admin_api::RegistrationEntry;

pub mod inmemory;

#[async_trait::async_trait]
pub trait Catalog: Sync + Send {
    type Error: std::error::Error + 'static;

    async fn get_registration_entry(&self, id: &str) -> Result<RegistrationEntry, Self::Error>;
    async fn create_registration_entry(&self, entry: RegistrationEntry) -> Result<(), Self::Error>;
    async fn update_registration_entry(&self, entry: RegistrationEntry) -> Result<(), Self::Error>;
    async fn delete_registration_entry(&self, id: &str) -> Result<(), Self::Error>;
    async fn list_registration_entries(
        &self,
        page_token: Option<String>,
        page_size: usize,
    ) -> Result<(Vec<RegistrationEntry>, Option<String>), Self::Error>;

    async fn add_key_to_jwt_trust_domain_store(
        &self,
        trust_domain: &str,
        kid: &str,
        public_key: PKey<Public>,
    ) -> Result<(), Self::Error>;
    async fn remove_key_jwt_trust_domain_store(
        &self,
        trust_domain: &str,
        kid: &str,
    ) -> Result<(), Self::Error>;
    async fn get_keys_from_jwt_trust_domain_store(
        &self,
        trust_domain: &str,
    ) -> Result<Vec<PKey<Public>>, Self::Error>;
}
