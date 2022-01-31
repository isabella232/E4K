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
}
