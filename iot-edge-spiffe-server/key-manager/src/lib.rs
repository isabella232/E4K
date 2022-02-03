// Copyright (c) Microsoft. All rights reserved.

mod error;

use std::{sync::Arc, time::SystemTime};
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

pub struct Manager<C: KeyPlugin + Send + Sync> {
    key_store : Arc<C>,
    jwt_key_type : KeyType,
    jwt_key_ttl : u64,
    current_jwt_key: Mutex<JWTKeyEntry>,
    next_jwt_key: Mutex<Option<JWTKeyEntry>>,
}

impl<C: KeyPlugin + Send + Sync> Manager<C> {
    pub async fn start(&'static mut self) {
        tokio::spawn(self.rotate_periodic());
        tokio::spawn(self.prunebundle());
    }

    async fn prunebundle(&self) {
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
    async fn rotate_periodic_logic(&self, current_time: u64) -> Result<(), Error> {
        let next_jwt_key = &mut *self.next_jwt_key.lock().await;
        let current_jwt_key = &mut *self.current_jwt_key.lock().await;

        let threshold = current_jwt_key.expiry - self.jwt_key_ttl/PREPARE_NEXT_KEY_FOR_ROTATION_MARGIN;


        if next_jwt_key.is_none() && (current_time > threshold)  {
            let id = Uuid::new_v4().to_string();
            self.key_store.create_key_pair_if_not_exists(&id, self.jwt_key_type)
            .await
            .map_err(|err| Error::ErrorCreatingNewKey(Box::new(err)))?;

            *next_jwt_key = Some(JWTKeyEntry { id, expiry: current_time + self.jwt_key_ttl });
        }

        let threshold = current_jwt_key.expiry - self.jwt_key_ttl/ROTATE_CURRENT_KEY_MARGIN;

        if current_time > threshold  {
            let jwt_key = next_jwt_key.clone().ok_or_else(Error::ErrorNextJwtKeyMissing)?;

            // We need to protect the bottom 2 operation, or another process 
            // could try to sign with the old key when it has already been deleted

            // Delete the old key
            self.key_store.delete_key_pair(&current_jwt_key.id)
            .await
            .map_err(|err| Error::ErrorDeletingOldKey(Box::new(err)))?;

            // Rotate keys,
            *current_jwt_key = jwt_key;
        }

        Ok(())
    }
}

fn get_epoch_time() -> u64 {
    let now = SystemTime::now();
    let epoch = now
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("Epoch should succeed");
    epoch.as_secs()
}