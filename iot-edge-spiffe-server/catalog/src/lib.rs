// Copyright (c) Microsoft. All rights reserved.

#![deny(rust_2018_idioms)]
#![warn(clippy::all, clippy::pedantic)]
#![allow(
    clippy::default_trait_access,
    clippy::let_unit_value,
    clippy::missing_errors_doc,
    clippy::similar_names,
    clippy::too_many_lines,
    clippy::missing_panics_doc
)]

use std::sync::Arc;

use core_objects::{RegistrationEntry, JWK};
use server_config::CatalogConfig;

pub mod inmemory;

pub struct CatalogFactory {}

impl CatalogFactory {
    #[must_use]
    pub fn get(config: &CatalogConfig) -> Arc<dyn Catalog> {
        match config {
            CatalogConfig::Disk => unimplemented!(),
            CatalogConfig::Memory => Arc::new(inmemory::Catalog::new()),
        }
    }
}

pub trait Catalog: Entries + TrustBundleStore {}

/// Entries are writen from the identity manager into the server. Entries contains all the necessary information
/// to identify a workload and issue a new about a SPIFFE identity to it.
#[async_trait::async_trait]
pub trait Entries: Sync + Send {
    /// Batch get registration entries
    ///
    /// ## Arguments
    /// * `ids` - ids of the entries.
    ///
    /// ## Returns
    /// * `Vec<(String, Result<RegistrationEntry, Error)>` - A vector the size of the input "ids". The first parameter
    /// of the tuple is the entryId, the second parameter is the entry if successful or an error
    async fn batch_get(
        &self,
        ids: &[String],
    ) -> Vec<(
        String,
        Result<RegistrationEntry, Box<dyn std::error::Error + Send>>,
    )>;

    /// Batch create registration entries
    ///
    /// ## Arguments
    /// * `Vec<RegistrationEntry>` -Vector containing all the ids to create.
    ///
    /// ## Returns
    /// * `Vec<(String, Result<((), Error)>` - A vector the size of the input "entries". The first parameter
    /// of the tuple is the entryId, the second parameter is () if successful or an error
    async fn batch_create(
        &self,
        entries: Vec<RegistrationEntry>,
    ) -> Result<(), Vec<(String, Box<dyn std::error::Error + Send>)>>;

    //Vec<(String, Result<(), Box<dyn std::error::Error + Send>>)>;

    /// Batch update registration entries
    ///
    /// ## Arguments
    /// * `Vec<RegistrationEntry>` -Vector containing all the ids to update.
    ///
    /// ## Returns
    /// * `Vec<(String, Result<(), Error)>` - A vector the size of the input "entries". The first parameter
    /// of the tuple is the entryId, the second parameter is () if successful or an error
    async fn batch_update(
        &self,
        entries: Vec<RegistrationEntry>,
    ) -> Result<(), Vec<(String, Box<dyn std::error::Error + Send>)>>;

    /// Batch delete registration entries
    ///
    /// ## Arguments
    /// * `ids` - ids of the entries.
    ///
    /// ## Returns
    /// * `Vec<(String, Result<(), Error)>` - A vector the size of the input "ids". The first parameter
    /// of the tuple is the entryId, the second parameter is () if successful or an error
    async fn batch_delete(
        &self,
        ids: &[String],
    ) -> Result<(), Vec<(String, Box<dyn std::error::Error + Send>)>>;

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
    ) -> Result<(Vec<RegistrationEntry>, Option<String>), Box<dyn std::error::Error + Send>>;

    /// Batch get registration entries
    ///
    /// ## Arguments
    /// * id of the entry
    ///
    /// ## Returns
    /// * Result<RegistrationEntry, Box<dyn std::error::Error + Send>>: The registration entry
    async fn get_entry(
        &self,
        id: &str,
    ) -> Result<RegistrationEntry, Box<dyn std::error::Error + Send>>;
}

/// The trust bundle store contains all the public keys necessary to validate  JWT tokens or trust certificates.
/// Those keys are writen by the key manager after a key rotation and read whenever the trust bundle is accessed.
/// The keys are sorted per trust domain.
#[async_trait::async_trait]
pub trait TrustBundleStore: Sync + Send {
    /// add a new public key for jwt in the catalog
    ///
    /// ## Arguments
    /// * `trust_domain` - trust domain for the key.
    /// * `jwk` - the jwk to add
    /// * `public_key` - public key.
    ///
    /// ## Returns
    /// * `Ok(())` - Successfully added the key
    /// * `Err(e)` - an error occurred while adding the key
    async fn add_jwk(
        &self,
        trust_domain: &str,
        jwk: JWK,
    ) -> Result<(), Box<dyn std::error::Error + Send>>;

    /// remove a public key for jwt from the catalog
    ///
    /// ## Arguments
    /// * `trust_domain` - trust domain for the key.
    /// * `kid` - unique key Id.
    ///
    /// ## Returns
    /// * `Ok(())` - Successfully deleted the key
    /// * `Err(e)` - an error occurred while deleting the key
    async fn remove_jwk(
        &self,
        trust_domain: &str,
        kid: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send>>;

    /// get all public keys for give trust domain
    ///
    /// ## Arguments
    /// * `trust_domain` - trust domain for the key.
    ///
    /// ## Returns
    /// * `Ok((JWK, usize))` - Array of JWK and the version number
    /// * `Err(e)` - an error occurred while getting the keys for the give trust domain    
    async fn get_jwk(
        &self,
        trust_domain: &str,
    ) -> Result<(Vec<JWK>, usize), Box<dyn std::error::Error + Send>>;
}
