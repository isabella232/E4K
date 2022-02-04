// Copyright (c) Microsoft. All rights reserved.

mod error;

use std::{sync::Arc, time::SystemTime};
use catalog::Catalog;
use log::error;
use error::Error;
use tokio::{time::{Duration, self}, sync::Mutex};
use key_store::{KeyPlugin, KeyType};
use uuid::Uuid;

const ROTATION_POLL_INTERVAL_SECONDS : u64 = 60;
// This is a divisor, so a higher divisor results in smaller margin
// This is the percentage of the lifetime of the current key left when the next key is created
const PREPARE_NEXT_KEY_FOR_ROTATION_MARGIN: u64 = 2;
// This is a divisor, so a higher divisor results in smaller margin
// This is the percentage of the lifetime of the current key left when the next key replaces the current key
const ROTATE_CURRENT_KEY_MARGIN: u64 = 6;

#[derive(Clone)]
struct JWTKeyEntry {
    id : String,
    expiry : u64, 
}

pub struct Manager<C, D> 
where
    C: KeyPlugin + Send + Sync,
    D: Catalog + Send + Sync
{
    trust_domain : String,
    catalog: Arc<D>,
    key_store : Arc<C>,
    jwt_key_type : KeyType,
    jwt_key_ttl : u64,
  
    previous_jwt_key_slot: Mutex<Option<JWTKeyEntry>>,
    current_jwt_key_slot: Mutex<JWTKeyEntry>,
    next_jwt_key_slot: Mutex<Option<JWTKeyEntry>>,
}

impl<C, D> Manager<C, D> 
where
    C: KeyPlugin + Send + Sync,
    D: Catalog + Send + Sync
{
    pub async fn start(&'static mut self) {
        tokio::spawn(self.rotate_periodic());
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

        let threshold = current_jwt_key.expiry - self.jwt_key_ttl/PREPARE_NEXT_KEY_FOR_ROTATION_MARGIN;

        // Create new key in the next slot. The pulic part of the key is added to the catalog.
        if next_jwt_key.is_none() && (current_time > threshold)  {
            let id = Uuid::new_v4().to_string();

            self.create_key_and_add_to_catalog(&id).await?;

            *next_jwt_key= Some(JWTKeyEntry { id: id.clone(), expiry: current_time + self.jwt_key_ttl });
        }

        let threshold = current_jwt_key.expiry - self.jwt_key_ttl/ROTATE_CURRENT_KEY_MARGIN;

        if current_time > threshold  {
            let jwt_key = next_jwt_key.clone().ok_or_else(Error::NextJwtKeyMissing)?;

            // Rotate keys, current key is the one used for signing.
            // This should never happen, the key should have expired a long time ago. But we clean up nonetheless and raise an error.
            if let Some(jwt_key) = previous_jwt_key {
                log::error!("Request of key current slot deprecation while key in previous slot has not expired yet");
                self.clean_jwt_key_slots(&jwt_key.id).await?;
            }
            *previous_jwt_key = Some(current_jwt_key.clone()); 
            *current_jwt_key = jwt_key;
            *next_jwt_key = None;
        }

        // If the key expire before being pushed out. It should not happen though.
        if let Some(jwt_key) = previous_jwt_key {
            if current_time > jwt_key.expiry {
                self.clean_jwt_key_slots(&jwt_key.id).await?;
                *previous_jwt_key = None;
            }
        }

        Ok(())
    }

    async fn clean_jwt_key_slots(&self, id: &str) -> Result<(), Error> {
        // Delete the old private key
        self.key_store.delete_key_pair(id)
        .await
        .map_err(|err| Error::DeletingPrivateKey(Box::new(err)))?;

        // Remove from catalog
        self.catalog.remove_key_jwt_trust_domain_store(&self.trust_domain, &id)
        .await
        .map_err(|err| Error::DeletingPublicKey(Box::new(err)))
    }

    async fn create_key_and_add_to_catalog(&self, id: &str) -> Result<(), Error> {
        self.key_store.create_key_pair_if_not_exists(&id, self.jwt_key_type)
        .await
        .map_err(|err| Error::CreatingNewKey(Box::new(err)))?;

        let public_key = self.key_store.get_public_key(&id).await
        .map_err(|err| Error::GettingPulicKey(Box::new(err)))?;
        
        self.catalog.add_key_to_jwt_trust_domain_store(&self.trust_domain, &id, public_key).await
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