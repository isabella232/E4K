// Copyright (c) Microsoft. All rights reserved.
#![deny(rust_2018_idioms)]
#![warn(clippy::all, clippy::pedantic)]
#![allow(
    clippy::missing_safety_doc,
    clippy::default_trait_access,
    clippy::let_unit_value,
    clippy::missing_errors_doc,
    clippy::similar_names,
    clippy::too_many_lines
)]

mod error;

use catalog::TrustBundleStore;
use common::{get_epoch_time, KeyType};
use config::Config;
use error::Error;
use key_store::KeyStore;
use log::error;
use std::sync::Arc;
use tokio::{
    sync::RwLock,
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

struct Slots {
    previous_jwt_key: Option<JWTKeyEntry>,
    current_jwt_key: JWTKeyEntry,
    next_jwt_key: Option<JWTKeyEntry>,
}

pub struct Manager<C, D>
where
    C: KeyStore + Send + Sync,
    D: TrustBundleStore + Send + Sync + 'static,
{
    trust_domain: String,
    catalog: Arc<D>,
    key_store: Arc<C>,
    jwt_key_type: KeyType,
    jwt_key_ttl: u64,

    slots: RwLock<Slots>,
}

impl<C, D> Manager<C, D>
where
    C: KeyStore + Send + Sync,
    D: TrustBundleStore + Send + Sync + 'static,
{
    pub async fn new(
        config: &Config,
        catalog: Arc<D>,
        key_store: Arc<C>,
        current_time: u64,
    ) -> Result<Self, Error> {
        let id = Uuid::new_v4().to_string();

        let jwt_key = JWTKeyEntry {
            id: id.clone(),
            expiry: current_time + config.jwt_key_ttl,
        };

        let slots = Slots {
            previous_jwt_key: None,
            current_jwt_key: jwt_key,
            next_jwt_key: None,
        };

        let key_manager = Manager::<C, D> {
            trust_domain: config.trust_domain.clone(),
            catalog,
            key_store,
            jwt_key_type: config.jwt_key_type,
            jwt_key_ttl: config.jwt_key_ttl,
            slots: RwLock::new(slots),
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
        let slots = &mut *self.slots.write().await;

        let threshold =
            slots.current_jwt_key.expiry - self.jwt_key_ttl / PREPARE_NEXT_KEY_FOR_ROTATION_MARGIN;

        // Create new key in the next slot. The pulic part of the key is added to the catalog.
        if slots.next_jwt_key.is_none() && (current_time > threshold) {
            let id = Uuid::new_v4().to_string();

            slots.next_jwt_key = Some(JWTKeyEntry {
                id: id.clone(),
                expiry: current_time + self.jwt_key_ttl,
            });

            self.create_key_and_add_to_catalog(&id).await?;
        }

        let threshold = slots.current_jwt_key.expiry - self.jwt_key_ttl / ROTATE_CURRENT_KEY_MARGIN;

        if current_time > threshold {
            let jwt_key = slots
                .next_jwt_key
                .clone()
                .ok_or_else(Error::NextJwtKeyMissing)?;

            // Rotate keys, current key is the one used for signing.
            // This should never happen, the key should have expired a long time ago. But we clean up nonetheless and raise an error.
            if let Some(jwt_key) = &slots.previous_jwt_key {
                log::error!("Request of key current slot deprecation while key in previous slot has not expired yet");
                self.remove_jwt_key_from_catalog_and_store(&jwt_key.id)
                    .await?;
            }
            slots.previous_jwt_key = Some(slots.current_jwt_key.clone());
            slots.current_jwt_key = jwt_key;
            slots.next_jwt_key = None;
        }

        // If the key expire before being pushed out. It should not happen though.
        if let Some(jwt_key) = &slots.previous_jwt_key {
            if current_time > jwt_key.expiry {
                self.remove_jwt_key_from_catalog_and_store(&jwt_key.id)
                    .await?;
                slots.previous_jwt_key = None;
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
            .remove_jwt_key(&self.trust_domain, id)
            .await
            .map_err(|err| Error::DeletingPublicKey(Box::new(err)))
    }

    async fn create_key_and_add_to_catalog(&self, id: &str) -> Result<(), Error> {
        let public_key = self
            .key_store
            .create_key_pair_if_not_exists(id, self.jwt_key_type)
            .await
            .map_err(|err| Error::CreatingNewKey(Box::new(err)))?;

        self.catalog
            .add_jwt_key(&self.trust_domain, id, public_key)
            .await
            .map_err(|err| Error::AddingPulicKey(Box::new(err)))
    }
}

#[cfg(test)]
mod tests {
    use catalog::{inmemory, TrustBundleStore};
    use config::{Config, KeyPluginConfigDisk};
    use key_store::{disk, KeyStore};
    use std::sync::Arc;
    use tempdir::TempDir;

    use crate::Manager;

    async fn init() -> (
        Manager<disk::KeyStore, inmemory::Catalog>,
        Arc<inmemory::Catalog>,
        Arc<disk::KeyStore>,
    ) {
        let mut config = Config::load_config(common::CONFIG_DEFAULT_PATH).unwrap();
        let dir = TempDir::new("test").unwrap();
        let key_base_path = dir.into_path().to_str().unwrap().to_string();
        let key_plugin = KeyPluginConfigDisk {
            key_base_path: key_base_path.clone(),
        };

        // Change key disk plugin path to write in tempdir
        config.key_plugin_disk = Some(key_plugin);
        // Force ttl to 300s
        config.jwt_key_ttl = 300;

        let catalog = Arc::new(inmemory::Catalog::new());
        let key_store = Arc::new(disk::KeyStore::new(
            &config.clone().key_plugin_disk.unwrap(),
        ));

        (
            Manager::new(&config, catalog.clone(), key_store.clone(), 0)
                .await
                .unwrap(),
            catalog,
            key_store,
        )
    }

    #[tokio::test]
    async fn initialize_test_happy_path() {
        let (manager, catalog, key_store) = init().await;

        // Check the public key has been uploaded
        let res = catalog.get_jwt_keys("dummy").await.unwrap();
        assert_eq!(res.len(), 1);

        // Check private key is in the store
        let current_jwt_key = &manager.slots.write().await.current_jwt_key;
        let _key = key_store.get_public_key(&current_jwt_key.id).await.unwrap();
    }

    #[tokio::test]
    async fn remove_jwt_key_from_catalog_and_store_test_happy_path() {
        let (manager, catalog, key_store) = init().await;

        let current_jwt_key = &manager.slots.write().await.current_jwt_key;
        manager
            .remove_jwt_key_from_catalog_and_store(&current_jwt_key.id)
            .await
            .unwrap();

        // Check it was removed from catalog
        let res = catalog.get_jwt_keys("dummy").await.unwrap();
        assert_eq!(res.len(), 0);

        // Check private key is in not the store
        let error = key_store
            .get_public_key(&current_jwt_key.id)
            .await
            .unwrap_err();
        if let disk::error::Error::KeyNotFound(_) = error {
        } else {
            panic!("Wrong error type returned for get_public_key")
        };
    }

    #[tokio::test]
    async fn rotate_periodic_test_state_machine() {
        let (manager, catalog, key_store) = init().await;

        // We test 3 events
        // 1. Next key create when current time > ttl/2
        // 2. key rotate (current->prev, next -> current) when current time > ttl - ttl/6
        // 5. key expiry time > ttl

        //------------------------ Stage 1 ----------------------------
        let (current_jwt_key_id, next_jwt_key_id) =
            run_stage1(&manager, catalog.clone(), key_store.clone()).await;

        //------------------------ Stage 2 ----------------------------
        run_stage2(&manager, &current_jwt_key_id, &next_jwt_key_id).await;

        //------------------------ Stage 3 ----------------------------
        run_stage3(&manager, catalog, key_store, &current_jwt_key_id).await;
    }

    async fn run_stage1(
        manager: &Manager<disk::KeyStore, inmemory::Catalog>,
        catalog: Arc<inmemory::Catalog>,
        key_store: Arc<disk::KeyStore>,
    ) -> (String, String) {
        manager
            .rotate_periodic_logic(manager.jwt_key_ttl / 2 + 1)
            .await
            .unwrap();
        let slots = &mut *manager.slots.write().await;

        assert!(slots.previous_jwt_key.is_none());

        let next_jwt_key_id = if let Some(next_jwt_key) = &slots.next_jwt_key {
            next_jwt_key.id.clone()
        } else {
            panic!("No next_jwt_key");
        };
        let current_jwt_key_id = slots.current_jwt_key.id.clone();

        // Now there should be 2 keys. One in the current slot, the other in the next.
        let res = catalog.get_jwt_keys("dummy").await.unwrap();
        assert_eq!(res.len(), 2);

        // Check private key is in the store
        let _key = key_store.get_public_key(&next_jwt_key_id).await.unwrap();

        (current_jwt_key_id, next_jwt_key_id)
    }

    async fn run_stage2(
        manager: &Manager<disk::KeyStore, inmemory::Catalog>,
        current_jwt_key_id: &str,
        next_jwt_key_id: &str,
    ) {
        manager
            .rotate_periodic_logic(manager.jwt_key_ttl - manager.jwt_key_ttl / 6 + 1)
            .await
            .unwrap();
        let slots = &mut *manager.slots.write().await;

        // Check key in current slot was moved to prev
        if let Some(prev_jwt_key) = &slots.previous_jwt_key {
            assert_eq!(prev_jwt_key.id, current_jwt_key_id);
        } else {
            panic!("No prev_jwt_key");
        };

        // Check key in next slot was moved to current
        assert_eq!(slots.current_jwt_key.id, next_jwt_key_id);

        //Check key has been removed from slot
        assert!(slots.next_jwt_key.is_none());
    }

    async fn run_stage3(
        manager: &Manager<disk::KeyStore, inmemory::Catalog>,
        catalog: Arc<inmemory::Catalog>,
        key_store: Arc<disk::KeyStore>,
        current_jwt_key_id: &str,
    ) {
        manager
            .rotate_periodic_logic(manager.jwt_key_ttl + 1)
            .await
            .unwrap();
        let prev_jwt_key = &manager.slots.write().await.previous_jwt_key;

        assert!(prev_jwt_key.is_none());

        // Now there should be only 1 keys. One in the current slot
        let res = catalog.get_jwt_keys("dummy").await.unwrap();
        assert_eq!(res.len(), 1);

        // Check private key is in the store
        let error = key_store
            .get_public_key(current_jwt_key_id)
            .await
            .unwrap_err();
        if let disk::error::Error::KeyNotFound(_) = error {
        } else {
            panic!("Wrong error type returned for get_public_key")
        };
    }
}
