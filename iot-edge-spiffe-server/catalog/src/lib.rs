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

/// Entries are writen from the identity manager into the server. Entries contains all the necessary information
/// to identify a workload and issue a new about a SPIFFE identity to it.
#[async_trait::async_trait]
pub trait Entries: Sync + Send {
    /// Entry error.
    type Error: std::error::Error + 'static;

    /// Get a registration entry
    ///
    /// ## Arguments
    /// * `id` - id of the entry.
    ///
    /// ## Returns
    /// * `Ok(RegistrationEntry)` - Successfully fetched the entry for the corresponding Id
    /// * `Err(e)` - an error occurred while getting the entry
    async fn get(&self, id: &str) -> Result<RegistrationEntry, Self::Error>;

    /// Create a registration entry
    ///
    /// ## Arguments
    /// * `RegistrationEntry` - The RegistrationEntry to create in the catalog
    ///
    /// ## Returns
    /// * `Ok(())` - Successfully created the entry
    /// * `Err(e)` - an error occurred while creating the entry  
    async fn create(&self, entry: RegistrationEntry) -> Result<(), Self::Error>;

    /// Update a registration entry
    ///
    /// ## Arguments
    /// * `RegistrationEntry` - The RegistrationEntry to update
    ///
    /// ## Returns
    /// * `Ok(())` - Successfully updated the entry
    /// * `Err(e)` - an error occurred while updating the entry     
    async fn update(&self, entry: RegistrationEntry) -> Result<(), Self::Error>;

    /// delete a registration entry
    ///
    /// ## Arguments
    /// * `id` - id of the entry.
    ///
    /// ## Returns
    /// * `Ok(())` - Successfully deleted the entry
    /// * `Err(e)` - an error occurred while deleting the entry  
    async fn delete(&self, id: &str) -> Result<(), Self::Error>;

    /// List all resgitration entries
    ///
    /// ## Arguments
    /// * `page_token` - page token, was returned from previous list_all(_) call.
    /// * `page_size` - how many request in the page.
    ///
    /// ## Returns
    /// * `Ok((Vec<RegistrationEntry>, Option<String>))` - All the entries in the requested page with the page token of the next page. If no more page, page_token is None.
    /// * `Err(e)` - an error occurred while trying to List all the entries
    async fn list_all(
        &self,
        page_token: Option<String>,
        page_size: usize,
    ) -> Result<(Vec<RegistrationEntry>, Option<String>), Self::Error>;
}

/// The trust bundle store contains all the public keys necessary to validate  JWT tokens or trust certificates.
/// Those keys are writen by the key manager after a key rotation and read whenever the trust bundle is accessed.
/// The keys are sorted per trust domain.
#[async_trait::async_trait]
pub trait TrustBundleStore: Sync + Send {
    /// Trust bundle error.
    type Error: std::error::Error + 'static;

    /// add a new public key for jwt in the catalog
    ///
    /// ## Arguments
    /// * `trust_domain` - trust domain for the key.
    /// * `kid` - unique key Id.
    /// * `public_key` - public key.
    ///
    /// ## Returns
    /// * `Ok(())` - Successfully added the key
    /// * `Err(e)` - an error occurred while adding the key
    async fn add_jwt_key(
        &self,
        trust_domain: &str,
        kid: &str,
        public_key: PKey<Public>,
    ) -> Result<(), Self::Error>;

    /// remove a public key for jwt from the catalog
    ///
    /// ## Arguments
    /// * `trust_domain` - trust domain for the key.
    /// * `kid` - unique key Id.
    ///
    /// ## Returns
    /// * `Ok(())` - Successfully deleted the key
    /// * `Err(e)` - an error occurred while deleting the key
    async fn remove_jwt_key(&self, trust_domain: &str, kid: &str) -> Result<(), Self::Error>;

    /// get all public keys for give trust domain
    ///
    /// ## Arguments
    /// * `trust_domain` - trust domain for the key.
    ///
    /// ## Returns
    /// * `Ok(Vec<PKey<Public>)` - Array of public keys
    /// * `Err(e)` - an error occurred while getting the keys for the give trust domain    
    async fn get_jwt_keys(&self, trust_domain: &str) -> Result<Vec<PKey<Public>>, Self::Error>;
}
