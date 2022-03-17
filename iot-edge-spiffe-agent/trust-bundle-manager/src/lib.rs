// Copyright (c) Microsoft. All rights reserved.

#![deny(rust_2018_idioms)]
#![warn(clippy::all, clippy::pedantic)]
#![allow(
    clippy::default_trait_access,
    clippy::let_unit_value,
    clippy::missing_errors_doc,
    clippy::similar_names,
    clippy::too_many_lines,
    clippy::missing_panics_doc
)]

pub mod error;

use std::{sync::Arc, time::Duration};

use agent_config::TrustBundleManagerConfig;
use core_objects::TrustBundle;
use error::Error;
use log::{info, warn};
use server_agent_api::get_trust_bundle;
use spiffe_server_client::Client;
use tokio::{sync::RwLock, time::sleep};

pub struct TrustBundleManager {
    trust_bundle: RwLock<TrustBundle>,
    spiffe_server_client: Arc<dyn Client>,
}

impl TrustBundleManager {
    #[must_use]
    pub fn new(spiffe_server_client: Arc<dyn Client>, init_trust_bundle: TrustBundle) -> Self {
        TrustBundleManager {
            trust_bundle: RwLock::new(init_trust_bundle),
            spiffe_server_client,
        }
    }

    pub async fn get_init_trust_bundle(
        spiffe_server_client: Arc<dyn Client>,
        config: &TrustBundleManagerConfig,
    ) -> Result<TrustBundle, Error> {
        info!("Getting first trust bundle");
        let mut retry = 0;

        loop {
            let params = get_trust_bundle::Params {
                jwt_keys: true,
                x509_cas: false,
            };

            let trust_bundle = spiffe_server_client.get_trust_bundle(params).await;

            match trust_bundle {
                Ok(trust_bundle) => return Ok(trust_bundle.trust_bundle),
                Err(err) => {
                    if retry >= config.max_retry {
                        return Err(Error::InitTrustBundle(err));
                    }
                    retry += 1;

                    warn!(
                        "Failed to get trust bundle {:?}, retrying {} out of {}",
                        err, retry, config.max_retry
                    );
                    sleep(Duration::from_secs(config.wait_retry_sec)).await;
                }
            }
        }
    }

    pub async fn refresh_trust_bundle(&self) -> Result<(), Error> {
        let params = get_trust_bundle::Params {
            jwt_keys: true,
            x509_cas: false,
        };

        let trust_bundle = &mut *self.trust_bundle.write().await;
        *trust_bundle = self
            .spiffe_server_client
            .get_trust_bundle(params)
            .await
            .map_err(Error::TrustBundle)?
            .trust_bundle;

        Ok(())
    }

    pub async fn get_cached_trust_bundle(&self) -> TrustBundle {
        self.trust_bundle.read().await.clone()
    }
}

#[cfg(test)]
mod tests {

    use std::sync::Arc;

    use agent_config::TrustBundleManagerConfig;
    use core_objects::{Crv, JWKSet, KeyUse, Kty, TrustBundle, JWK};
    use matches::assert_matches;
    use server_agent_api::get_trust_bundle;
    use spiffe_server_client::MockClient;

    use crate::{error::Error, TrustBundleManager};

    #[tokio::test]
    async fn get_init_trust_bundle_happy_path() {
        let mut mock_client = MockClient::new();

        let expected_init_trust_bundle = get_trust_bundle();

        let config = TrustBundleManagerConfig {
            max_retry: 3,
            wait_retry_sec: 0,
        };

        mock_client.expect_get_trust_bundle().return_once(|_| {
            Ok(get_trust_bundle::Response {
                trust_bundle: get_trust_bundle(),
            })
        });

        let trust_bundle =
            TrustBundleManager::get_init_trust_bundle(Arc::new(mock_client), &config)
                .await
                .unwrap();

        assert_eq!(
            trust_bundle.trust_domain,
            expected_init_trust_bundle.trust_domain
        );
        assert_eq!(
            trust_bundle.jwt_key_set.keys[0].x,
            expected_init_trust_bundle.jwt_key_set.keys[0].x
        );

        assert_eq!(
            trust_bundle.jwt_key_set.keys[0].x,
            expected_init_trust_bundle.jwt_key_set.keys[0].x
        );
    }

    #[tokio::test]
    async fn get_init_trust_bundle_error() {
        let mut mock_client = MockClient::new();

        let config = TrustBundleManagerConfig {
            max_retry: 3,
            wait_retry_sec: 0,
        };

        mock_client
            .expect_get_trust_bundle()
            .times(4)
            .returning(|_| {
                Err(Box::new(
                    spiffe_server_client::http::error::Error::Connector("dummy".to_string()),
                ))
            });

        let error = TrustBundleManager::get_init_trust_bundle(Arc::new(mock_client), &config)
            .await
            .unwrap_err();

        assert_matches!(error, Error::InitTrustBundle(_));
    }

    #[tokio::test]
    async fn refresh_trust_bundle_error_path() {
        let mut mock_client = MockClient::new();

        let expected_init_trust_bundle = get_trust_bundle();

        mock_client.expect_get_trust_bundle().return_once(|_| {
            Err(Box::new(
                spiffe_server_client::http::error::Error::Connector("dummy".to_string()),
            ))
        });

        let trust_bundle_manager =
            TrustBundleManager::new(Arc::new(mock_client), expected_init_trust_bundle.clone());

        let error = trust_bundle_manager
            .refresh_trust_bundle()
            .await
            .unwrap_err();

        assert_matches!(error, Error::TrustBundle(_));
    }

    #[tokio::test]
    async fn refresh_trust_bundle_and_get_cached_trust_bundle_happy_path() {
        let mut mock_client = MockClient::new();

        let expected_trust_bundle1 = get_trust_bundle();

        // Then return a different trust bundle
        let mut expected_trust_bundle2 = get_trust_bundle();
        expected_trust_bundle2.jwt_key_set.keys[0].x = "1234".to_string();
        let expected_trust_bundle_copy = expected_trust_bundle2.clone();
        mock_client.expect_get_trust_bundle().return_once(move |_| {
            Ok(get_trust_bundle::Response {
                trust_bundle: expected_trust_bundle_copy,
            })
        });

        let trust_bundle_manager =
            TrustBundleManager::new(Arc::new(mock_client), expected_trust_bundle1.clone());

        let trust_bundle = trust_bundle_manager.get_cached_trust_bundle().await;
        assert_eq!(
            trust_bundle.jwt_key_set.keys[0].x,
            expected_trust_bundle1.jwt_key_set.keys[0].x
        );

        // Refresh trust bundle
        trust_bundle_manager.refresh_trust_bundle().await.unwrap();
        // Get new trust bundle
        let trust_bundle = trust_bundle_manager.get_cached_trust_bundle().await;
        // key should now match 1234
        assert_eq!(
            trust_bundle.jwt_key_set.keys[0].x,
            expected_trust_bundle2.jwt_key_set.keys[0].x
        );
    }

    fn get_trust_bundle() -> TrustBundle {
        let jwk = JWK {
            x: "MjE2NDE3NTMwMTgxMjY5Njc2MTE3MzAwODU4NjY4Mjg2MDU4MTQ2OTY3ODY0MjU2MDA1MzI0NTA0ODQyNTcxMTcyMzI4NjM1MjgxMjM".to_string(),
            y: "MzU1NjA3MjI0Mjc5MzAxMjYzMzkxNDg5NjAxMDA2NjMzNDE1NTA2MzQzMTQ5MDIxNzQxNTI0MDMyMzk0ODA1NjM2NjE0MTU0NjMyNzI".to_string(),
            kty: Kty::EC,
            crv: Crv::P256,
            kid: "kid".to_string(),
            key_use: KeyUse::JWTSVID,
        };

        TrustBundle {
            trust_domain: "trust_domain".to_string(),
            jwt_key_set: JWKSet {
                keys: vec![jwk],
                spiffe_refresh_hint: 0,
                spiffe_sequence_number: 0,
            },
            x509_key_set: JWKSet {
                keys: Vec::new(),
                spiffe_refresh_hint: 0,
                spiffe_sequence_number: 0,
            },
        }
    }
}
