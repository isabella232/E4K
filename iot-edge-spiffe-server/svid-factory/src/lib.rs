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

pub mod error;

use std::{cmp::min, sync::Arc};

use core_objects::{
    get_epoch_time, IdentityTypes, JWTClaims, JWTHeader, JWTSVIDCompact, JWTType, SPIFFEID,
};
use error::Error;
use key_manager::KeyManager;
use openssl::sha;
use server_config::Config;
pub struct SVIDFactory {
    key_manager: Arc<KeyManager>,
    jwt_ttl: u64,
}

#[derive(Clone)]
pub struct JWTSVIDParams {
    pub spiffe_id: SPIFFEID,
    pub audiences: Vec<SPIFFEID>,
    pub other_identities: Vec<IdentityTypes>,
}

impl SVIDFactory {
    #[must_use]
    pub fn new(key_manager: Arc<KeyManager>, config: &Config) -> Self {
        SVIDFactory {
            key_manager,
            jwt_ttl: config.jwt.ttl,
        }
    }

    pub async fn create_jwt_svid(
        &self,
        jwt_svid_params: JWTSVIDParams,
    ) -> Result<JWTSVIDCompact, Error> {
        let issued_at = get_epoch_time();

        self.create_jwt_svid_inner(jwt_svid_params, issued_at).await
    }

    async fn create_jwt_svid_inner(
        &self,
        jwt_svid_params: JWTSVIDParams,
        issued_at: u64,
    ) -> Result<JWTSVIDCompact, Error> {
        let slots = &*self.key_manager.slots.read().await;
        let jwt_key = &slots.current_jwt_key;

        let expiry = issued_at + self.jwt_ttl;
        // Do not generate an svid with a lifetime bigger than the private key.
        let expiry = min(expiry, jwt_key.expiry);

        let header = JWTHeader {
            algorithm: self.key_manager.jwt_key_type,
            key_id: jwt_key.id.clone(),
            jwt_type: JWTType::JWT,
        };

        let claims = JWTClaims {
            subject: jwt_svid_params.spiffe_id.clone(),
            audience: jwt_svid_params.audiences,
            expiry,
            issued_at,
            other_identities: jwt_svid_params.other_identities,
        };

        let header_compact = serde_json::to_string(&header).map_err(Error::ErrorJSONSerializing)?;
        let header_compact =
            base64::encode_config(header_compact.as_bytes(), base64::STANDARD_NO_PAD);

        let claims_compact = serde_json::to_string(&claims).map_err(Error::ErrorJSONSerializing)?;
        let claims_compact =
            base64::encode_config(claims_compact.as_bytes(), base64::STANDARD_NO_PAD);

        let signature = format!("{}.{}", header_compact, claims_compact);

        let signature = match self.key_manager.jwt_key_type {
            core_objects::KeyType::ES256 => sha::sha256(signature.as_bytes()),
            _ => return Err(Error::UnimplementedKeyType(self.key_manager.jwt_key_type)),
        };

        let signature = self
            .key_manager
            .key_store
            .sign(&jwt_key.id, self.key_manager.jwt_key_type, &signature)
            .await
            .map_err(Error::SigningDigest)?;

        let signature = base64::encode_config(signature.1, base64::STANDARD_NO_PAD);
        let token = format!("{}.{}.{}", header_compact, claims_compact, signature);

        Ok(JWTSVIDCompact {
            token,
            spiffe_id: jwt_svid_params.spiffe_id,
            expiry,
            issued_at,
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
    use matches::assert_matches;
    use server_config::{Config, KeyStoreConfig, KeyStoreConfigDisk};
    use std::sync::Arc;
    use tempdir::TempDir;

    async fn init() -> (SVIDFactory, Config) {
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

        let key_manager = Arc::new(
            KeyManager::new(&config, catalog.clone(), key_store.clone(), 0)
                .await
                .unwrap(),
        );

        (SVIDFactory::new(key_manager, &config), config)
    }

    #[tokio::test]
    async fn sign_digest_happy_path() {
        let (svid_factory, config) = init().await;

        let spiffe_id = SPIFFEID {
            trust_domain: "trust_domain".to_string(),
            path: "path".to_string(),
        };

        let jwt_svid_params = JWTSVIDParams {
            spiffe_id: spiffe_id.clone(),
            audiences: vec![SPIFFEID {
                trust_domain: "my trust domain".to_string(),
                path: "audiences".to_string(),
            }],
            other_identities: Vec::new(),
        };

        let jwt_svid = svid_factory
            .create_jwt_svid_inner(jwt_svid_params, 0)
            .await
            .unwrap();

        assert_eq!(config.jwt.ttl, jwt_svid.expiry);

        assert_eq!(spiffe_id.to_string(), jwt_svid.spiffe_id.to_string());
    }

    #[tokio::test]
    async fn sign_digest_saturation_test() {
        let (svid_factory, config) = init().await;

        let spiffe_id = SPIFFEID {
            trust_domain: "trust_domain".to_string(),
            path: "path".to_string(),
        };

        let jwt_svid_params = JWTSVIDParams {
            spiffe_id: spiffe_id.clone(),
            audiences: vec![SPIFFEID {
                trust_domain: "my trust domain".to_string(),
                path: "audiences".to_string(),
            }],
            other_identities: Vec::new(),
        };

        // Generate an SVID close to the key expiration. The expiry time should not be after the expiration.
        let jwt_svid = svid_factory
            .create_jwt_svid_inner(jwt_svid_params, config.jwt.key_ttl - 1)
            .await
            .unwrap();

        assert_eq!(config.jwt.key_ttl, jwt_svid.expiry);
    }

    #[tokio::test]
    async fn sign_digest_error_path() {
        let (svid_factory, _config) = init().await;
        let manager = svid_factory.key_manager.clone();

        {
            let current_jwt_key = &manager.slots.read().await.current_jwt_key;
            let id = current_jwt_key.clone().id;
            manager.key_store.delete_key_pair(&id).await.unwrap();
        }

        let spiffe_id = SPIFFEID {
            trust_domain: "trust_domain".to_string(),
            path: "path".to_string(),
        };

        let jwt_svid_params = JWTSVIDParams {
            spiffe_id,
            audiences: vec![SPIFFEID {
                trust_domain: "my trust domain".to_string(),
                path: "audiences".to_string(),
            }],
            other_identities: Vec::new(),
        };

        let error = svid_factory
            .create_jwt_svid(jwt_svid_params)
            .await
            .unwrap_err();
        assert_matches!(error, Error::SigningDigest(_));
    }
}
