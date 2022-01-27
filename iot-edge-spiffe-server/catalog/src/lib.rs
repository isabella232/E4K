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

use common_admin_api::RegistrationEntry;
use error::Error;

mod error;
mod inmemory;

#[must_use]
pub fn load_catalog() -> Box<dyn Catalog + Send + Sync> {
    Box::new(inmemory::InMemoryCatalog::new())
}

#[async_trait::async_trait]
pub trait Catalog: Sync + Send {
    async fn get_registration_entry(&self, id: &str) -> Result<RegistrationEntry, Error>;
    async fn create_registration_entry(&mut self, entry: RegistrationEntry) -> Result<(), Error>;
    async fn update_registration_entry(&mut self, entry: RegistrationEntry) -> Result<(), Error>;
    async fn delete_registration_entry(&mut self, id: &str) -> Result<(), Error>;
    async fn list_registration_entries(
        &self,
        page_number: i32,
        page_size: i32,
    ) -> Result<(Vec<RegistrationEntry>, i32), Error>;
}
