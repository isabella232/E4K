// Copyright (c) Microsoft. All rights reserved.

use core_objects::RegistrationEntry;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync + 'static>>;

#[async_trait::async_trait]
pub trait SpiffeConnector {
    async fn get_identities(&self) -> Result<Vec<RegistrationEntry>>;
    async fn create_identities(&self, identities_to_create: Vec<RegistrationEntry>) -> Result<()>;
    async fn delete_identities(&self, identities_to_delete: Vec<String>) -> Result<()>;
}
