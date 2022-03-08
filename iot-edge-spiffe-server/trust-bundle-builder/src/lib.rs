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

use std::sync::Arc;

use catalog::Catalog;
use core_objects::{JWKSet, TrustBundle};
use error::Error;
use server_config::Config;

pub mod error;

pub struct TrustBundleBuilder {
    trust_domain: String,
    refresh_hint: u64,
    catalog: Arc<dyn Catalog + Sync + Send>,
}

impl TrustBundleBuilder {
    #[must_use]
    pub fn new(config: &Config, catalog: Arc<dyn Catalog + Sync + Send>) -> Arc<Self> {
        Arc::new(TrustBundleBuilder {
            trust_domain: config.trust_domain.clone(),
            refresh_hint: config.trust_bundle.refresh_hint,
            catalog,
        })
    }

    pub async fn build_trust_bundle(
        &self,
        jwt_keys: bool,
        _x509_cas: bool,
    ) -> Result<TrustBundle, Error> {
        let (jwt_key, version) = if jwt_keys {
            self.catalog
                .get_jwk(&self.trust_domain)
                .await
                .map_err(Error::CatalogGetKeys)?
        } else {
            (Vec::new(), 0)
        };

        let jwt_key_set = JWKSet {
            keys: jwt_key,
            spiffe_refresh_hint: self.refresh_hint,
            spiffe_sequence_number: version as u64,
        };

        let x509_key_set = JWKSet {
            keys: Vec::new(),
            spiffe_refresh_hint: self.refresh_hint,
            spiffe_sequence_number: version as u64,
        };

        Ok(TrustBundle {
            trust_domain: self.trust_domain.to_string(),
            jwt_key_set,
            x509_key_set,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use catalog::inmemory;
    use core_objects::CONFIG_DEFAULT_PATH;
    use key_manager::KeyManager;
    use key_store::disk;
    use server_config::{Config, KeyStoreConfig, KeyStoreConfigDisk};

    use std::sync::Arc;
    use tempdir::TempDir;

    async fn init() -> (Arc<TrustBundleBuilder>, Config, KeyManager) {
        let mut config = Config::load_config(CONFIG_DEFAULT_PATH).unwrap();
        let dir = TempDir::new("test").unwrap();
        let key_base_path = dir.into_path().to_str().unwrap().to_string();
        let key_plugin = KeyStoreConfigDisk {
            key_base_path: key_base_path.clone(),
        };

        // Change key disk plugin path to write in tempdir
        config.key_store = KeyStoreConfig::Disk(key_plugin.clone());
        // Force ttl to 300s
        config.jwt.key_ttl = 300;

        let catalog = Arc::new(inmemory::Catalog::new());
        let key_store = Arc::new(disk::KeyStore::new(&key_plugin));

        let key_manager = KeyManager::new(&config, catalog.clone(), key_store.clone(), 0)
            .await
            .unwrap();

        (
            TrustBundleBuilder::new(&config, catalog),
            config,
            key_manager,
        )
    }

    #[tokio::test]
    async fn build_trust_bundle_happy_path() {
        let (trust_bundle_builder, config, key_manager) = init().await;

        let slots = key_manager.slots.read().await;
        let id = slots.current_jwt_key.id.clone();

        let trust_bundle = trust_bundle_builder
            .build_trust_bundle(true, false)
            .await
            .unwrap();

        let jwk = &trust_bundle.jwt_key_set.keys[0];

        assert_eq!(1, trust_bundle.jwt_key_set.keys.len());
        assert_eq!(config.trust_domain, trust_bundle.trust_domain);
        assert_eq!(
            config.trust_bundle.refresh_hint,
            trust_bundle.jwt_key_set.spiffe_refresh_hint
        );
        assert_eq!(id, jwk.kid);

        let trust_bundle = trust_bundle_builder
            .build_trust_bundle(false, false)
            .await
            .unwrap();
        assert_eq!(0, trust_bundle.jwt_key_set.keys.len());
    }
}
