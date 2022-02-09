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

use catalog::{Entries, TrustBundleStore};
use config::Config;
use error::Error;
use http_common::Connector;
use key_manager::KeyManager;
use key_store::KeyStore;
use server_agent_api::{create_new_jwt, get_trust_bundle, Bundle, JWTSVID, SPIFFEID};
use std::{io, sync::Arc};

mod error;
mod http;

const SOCKET_DEFAULT_PERMISSION: u32 = 0o660;

pub async fn start_server_api<C, D>(
    config: &Config,
    catalog: Arc<C>,
    key_manager: Arc<KeyManager<C, D>>,
) -> Result<(), io::Error>
where
    C: Entries + TrustBundleStore + Send + Sync + 'static,
    D: KeyStore + Send + Sync + 'static,
{
    let api = Api::<C, D> {
        catalog,
        key_manager,
    };

    let service = http::Service { api };

    let connector = Connector::Unix {
        socket_path: std::path::Path::new(&config.socket_path).into(),
    };

    let mut incoming = connector.incoming(SOCKET_DEFAULT_PERMISSION, None).await?;

    // Channel to gracefully shut down the server. It's currently not used.
    let (_shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();

    let () = incoming.serve(service, shutdown_rx).await?;

    log::info!("Stopped server.");

    Ok(())
}

pub mod uri {
    pub const CREATE_NEW_JTW: &str = "/new-JWT-SVID";
    pub const GET_TRUST_BUNDLE: &str = "/trust-bundle";
}

struct Api<C, D>
where
    C: Entries + TrustBundleStore + Send + Sync + 'static,
    D: KeyStore + Send + Sync + 'static,
{
    catalog: Arc<C>,
    key_manager: Arc<KeyManager<C, D>>,
}

impl<C, D> Api<C, D>
where
    C: Entries + TrustBundleStore + Send + Sync + 'static,
    D: KeyStore + Send + Sync + 'static,
{
    pub async fn create_new_jwt(&self, req: create_new_jwt::Request) -> create_new_jwt::Response {
        let _catalog_results = self.catalog.batch_get(&[req.id].to_vec()).await;

        let digest = "hello world".as_bytes();
        self.key_manager
            .sign_jwt_with_current_key(digest)
            .await
            .unwrap();

        let _dummy = Error::DummyError("test".to_string());

        // Create dummy response
        create_new_jwt::Response {
            jwt_svid: JWTSVID {
                token: "dummy".to_string(),
                spiffe_id: SPIFFEID {
                    trust_domain: "dummy".to_string(),
                    path: "dummy".to_string(),
                },
                expire_at: 0,
                issued_at: 0,
            },
        }
    }

    pub async fn get_trust_bundle(
        &self,
        _params: get_trust_bundle::Params,
    ) -> get_trust_bundle::Response {
        // Create dummy response
        get_trust_bundle::Response {
            bundle: Bundle {
                trust_domain: "dummy".to_string(),
                jwt_keys: Vec::new(),
                x509_cas: Vec::new(),
                refresh_hint: 0,
                sequence_number: 0,
            },
        }
    }
}

impl<C, D> Clone for Api<C, D>
where
    C: Entries + TrustBundleStore + Send + Sync + 'static,
    D: KeyStore + Send + Sync + 'static,
{
    fn clone(&self) -> Self {
        Self {
            catalog: self.catalog.clone(),
            key_manager: self.key_manager.clone(),
        }
    }
}
