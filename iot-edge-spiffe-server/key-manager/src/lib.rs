// Copyright (c) Microsoft. All rights reserved.

mod error;

use catalog::Catalog;
use common::KeyType;
use config::Config;
use error::Error;
use key_store::KeyStore;
use log::error;
use std::{sync::Arc, time::SystemTime};
use tokio::{
    sync::Mutex,
    task::JoinHandle,
    time::{self, Duration},
};
use uuid::Uuid;

const ROTATION_POLL_INTERVAL_SECONDS: u64 = 60;
// This is a divisor, so a higher divisor results in smaller margin
// This is the percentage of the lifetime of the current key left when the next key is created
const PREPARE_NEXT_KEY_FOR_ROTATION_MARGIN: u64 = 2;
// This is a divisor, so a higher divisor results in smaller margin
// This is the percentage of the lifetime of the current key left when the next key replaces the current key
const ROTATE_CURRENT_KEY_MARGIN: u64 = 6;

#[derive(Clone)]
struct JWTKeyEntry {
    id: String,
    expiry: u64,
}

pub struct Manager<C, D>
where
    C: KeyStore + Send + Sync,
    D: Catalog + Send + Sync,
{
    trust_domain: String,
    catalog: Arc<D>,
    key_store: Arc<C>,
    jwt_key_type: KeyType,
    jwt_key_ttl: u64,

    previous_jwt_key_slot: Mutex<Option<JWTKeyEntry>>,
    current_jwt_key_slot: Mutex<JWTKeyEntry>,
    next_jwt_key_slot: Mutex<Option<JWTKeyEntry>>,
}

impl<C, D> Manager<C, D>
where
    C: KeyStore + Send + Sync,
    D: Catalog + Send + Sync,
{
    pub async fn new(config: &Config, catalog: Arc<D>, key_store: Arc<C>) -> Result<Self, Error> {
        let id = Uuid::new_v4().to_string();
        let current_time = get_epoch_time();
        let jwt_key = JWTKeyEntry {
            id: id.clone(),
            expiry: current_time + config.jwt_key_ttl,
        };

        let key_manager = Manager::<C, D> {
            trust_domain: config.trust_domain.clone(),
            catalog,
            key_store,
            jwt_key_type: config.jwt_key_type,
            jwt_key_ttl: config.jwt_key_ttl,
            previous_jwt_key_slot: Mutex::new(None),
            current_jwt_key_slot: Mutex::new(jwt_key),
            next_jwt_key_slot: Mutex::new(None),
        };

        key_manager.create_key_and_add_to_catalog(&id).await?;

        Ok(key_manager)
    }

    pub async fn start(&'static self) -> JoinHandle<()> {
        tokio::spawn(self.rotate_periodic())
    }

    async fn rotate_periodic(&self) {
        let mut interval = time::interval(Duration::from_secs(ROTATION_POLL_INTERVAL_SECONDS));

        loop {
            interval.tick().await;

            let current_time = get_epoch_time();
            if let Err(err) = self.rotate_periodic_logic(current_time).await {
                error!("{}", err);
            }
        }
    }

    // Separated logic from rotate_periodic to be able to unit test it
    // At the beginning we have only current_key
    // Then some time later, we create the next_key. This key is not used for signing yet, but its public key is added to the trust bundle.
    // Then again some time later, once we are confident that trust bundle as been propagated to the workloads, we stop using the current key
    // and start using the next key for signing. We move current key to sleep in previous and next key to active in current.
    // Then some more time later, when the previous key expire, it is destroyed.
    async fn rotate_periodic_logic(&self, current_time: u64) -> Result<(), Error> {
        let next_jwt_key = &mut *self.next_jwt_key_slot.lock().await;
        let current_jwt_key = &mut *self.current_jwt_key_slot.lock().await;
        let previous_jwt_key = &mut *self.previous_jwt_key_slot.lock().await;

        let threshold =
            current_jwt_key.expiry - self.jwt_key_ttl / PREPARE_NEXT_KEY_FOR_ROTATION_MARGIN;

        // Create new key in the next slot. The pulic part of the key is added to the catalog.
        if next_jwt_key.is_none() && (current_time > threshold) {
            let id = Uuid::new_v4().to_string();

            *next_jwt_key = Some(JWTKeyEntry {
                id: id.clone(),
                expiry: current_time + self.jwt_key_ttl,
            });

            self.create_key_and_add_to_catalog(&id).await?;
        }

        let threshold = current_jwt_key.expiry - self.jwt_key_ttl / ROTATE_CURRENT_KEY_MARGIN;

        if current_time > threshold {
            let jwt_key = next_jwt_key.clone().ok_or_else(Error::NextJwtKeyMissing)?;

            // Rotate keys, current key is the one used for signing.
            // This should never happen, the key should have expired a long time ago. But we clean up nonetheless and raise an error.
            if let Some(jwt_key) = previous_jwt_key {
                log::error!("Request of key current slot deprecation while key in previous slot has not expired yet");
                self.remove_jwt_key_from_catalog_and_store(&jwt_key.id).await?;
            }
            *previous_jwt_key = Some(current_jwt_key.clone());
            *current_jwt_key = jwt_key;
            *next_jwt_key = None;
        }

        // If the key expire before being pushed out. It should not happen though.
        if let Some(jwt_key) = previous_jwt_key {
            if current_time > jwt_key.expiry {
                self.remove_jwt_key_from_catalog_and_store(&jwt_key.id).await?;
                *previous_jwt_key = None;
            }
        }

        Ok(())
    }

    async fn remove_jwt_key_from_catalog_and_store(&self, id: &str) -> Result<(), Error> {
        // Delete the old private key
        self.key_store
            .delete_key_pair(id)
            .await
            .map_err(|err| Error::DeletingPrivateKey(Box::new(err)))?;

        // Remove from catalog
        self.catalog
            .remove_key_jwt_trust_domain_store(&self.trust_domain, id)
            .await
            .map_err(|err| Error::DeletingPublicKey(Box::new(err)))
    }

    async fn create_key_and_add_to_catalog(&self, id: &str) -> Result<(), Error> {
        self.key_store
            .create_key_pair_if_not_exists(id, self.jwt_key_type)
            .await
            .map_err(|err| Error::CreatingNewKey(Box::new(err)))?;

        let public_key = self
            .key_store
            .get_public_key(id)
            .await
            .map_err(|err| Error::GettingPulicKey(Box::new(err)))?;

        self.catalog
            .add_key_to_jwt_trust_domain_store(&self.trust_domain, id, public_key)
            .await
            .map_err(|err| Error::AddingPulicKey(Box::new(err)))
    }
}

fn get_epoch_time() -> u64 {
    let now = SystemTime::now();
    let epoch = now
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("Epoch should succeed");
    epoch.as_secs()
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use tempdir::TempDir;
    use catalog::{inmemory, Catalog};
    use config::{Config, KeyPluginConfigDisk};
    use key_store::{KeyStore, disk};

    use crate::Manager;

    async fn init() -> (Manager<disk::KeyStore, inmemory::Catalog>, Arc<inmemory::Catalog>, Arc<disk::KeyStore>) {

        let mut config = Config::load_config(common::CONFIG_DEFAULT_PATH).unwrap();
        let dir = TempDir::new("test").unwrap();
        let key_base_path = dir.into_path().to_str().unwrap().to_string();
        let key_plugin = KeyPluginConfigDisk {
            key_base_path: key_base_path.clone(),
        };
        
        // Change key disk plugin path to write in tempdir
        config.key_plugin_disk = Some(key_plugin);

        let catalog = Arc::new(inmemory::Catalog::new());
        let key_store = Arc::new(disk::KeyStore::new(&config.clone().key_plugin_disk.unwrap()));

        (Manager::new(&config, catalog.clone(), key_store.clone()).await.unwrap(), catalog, key_store)
    }

    #[tokio::test]
    async fn initialize_test_happy_path() {
        let (manager, catalog, key_store) = init().await;

        // Check the public key has been uploaded
        let res = catalog.get_keys_from_jwt_trust_domain_store("dummy").await.unwrap();
        assert_eq!(res.len(), 1);

        // Check private key is in the store
        let next_jwt_key = manager.current_jwt_key_slot.lock().await;
        let _key = key_store.get_public_key(&next_jwt_key.id).await.unwrap();
    }

    #[tokio::test]
    async fn remove_jwt_key_from_catalog_and_store_test_happy_path() {
        let (manager, catalog, key_store) = init().await;

        let next_jwt_key = manager.current_jwt_key_slot.lock().await;
        manager.remove_jwt_key_from_catalog_and_store(&next_jwt_key.id).await.unwrap();

        // Check it was removed from catalog
        let res = catalog.get_keys_from_jwt_trust_domain_store("dummy").await.unwrap();
        assert_eq!(res.len(), 0);

        // Check private key is in not the store
        let error = key_store.get_public_key(&next_jwt_key.id).await.unwrap_err();
        if let disk::error::Error::KeyNotFound(_) = error {
        } else {
            panic!("Wrong error type returned for get_public_key")
        };
    }   
}
