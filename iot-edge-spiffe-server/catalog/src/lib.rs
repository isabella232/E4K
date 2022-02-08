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

// Entries are writen from the identity manager into the server. Entries contains all the necessary information
// to identify a workload and issue a new about a SPIFFE identity to it.
#[async_trait::async_trait]
pub trait Entries: Sync + Send {
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
}

// The trust bundle store contains all the public keys necessary to validate  JWT tokens or trust certificates.
// Those keys are writen by the key manager after a key rotation and read whenever the trust bundle is accessed.
// The keys are sorted per trust domain.
#[async_trait::async_trait]
pub trait TrustBundleStore: Sync + Send {
    type Error: std::error::Error + 'static;

    async fn add_jwt_key(
        &self,
        trust_domain: &str,
        kid: &str,
        public_key: PKey<Public>,
    ) -> Result<(), Self::Error>;
    async fn remove_jwt_key(&self, trust_domain: &str, kid: &str) -> Result<(), Self::Error>;
    async fn get_jwt_keys(&self, trust_domain: &str) -> Result<Vec<PKey<Public>>, Self::Error>;
}
