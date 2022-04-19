// Copyright (c) Microsoft. All rights reserved.

use std::sync::Mutex;

use core_objects::RegistrationEntry;

use super::SpiffeConnector;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync + 'static>>;

#[derive(Default)]
pub struct SpiffeFakeConnector {
    pub current_identities: Mutex<Vec<RegistrationEntry>>,
    pub added_identities: Mutex<Vec<RegistrationEntry>>,
    pub removed_identities: Mutex<Vec<String>>,
}

#[async_trait::async_trait]
impl SpiffeConnector for SpiffeFakeConnector {
    async fn get_identities(&self) -> Result<Vec<RegistrationEntry>> {
        let current_identities = self.current_identities.lock().unwrap();
        Ok(current_identities.clone())
    }

    async fn create_identities(&self, identities_to_create: Vec<RegistrationEntry>) -> Result<()> {
        let mut current_identities = self.current_identities.lock().unwrap();
        let mut added_identities = self.added_identities.lock().unwrap();

        for identity in identities_to_create {
            current_identities.push(identity.clone());
            added_identities.push(identity);
        }

        Ok(())
    }

    async fn delete_identities(&self, identities_to_delete: Vec<String>) -> Result<()> {
        let mut current_identities = self.current_identities.lock().unwrap();
        let mut removed_identities = self.removed_identities.lock().unwrap();

        for identity in identities_to_delete {
            current_identities.retain(|i| i.id != identity);
            removed_identities.push(identity);
        }

        Ok(())
    }
}
