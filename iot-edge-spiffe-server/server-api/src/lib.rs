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

use error::Error;
use http_common::Connector;
use server_agent_api::{create_new_jwt, get_trust_bundle, Bundle, JWTSVID, SPIFFEID};
use server_config::Config;
use std::io;

mod error;
mod http;

const SOCKET_DEFAULT_PERMISSION: u32 = 0o660;

pub async fn start_server_api(config: &Config) -> Result<(), io::Error> {
    let api = Api {};

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

#[derive(Clone)]
struct Api {}

impl Api {
    pub async fn create_new_jwt(&self, _req: create_new_jwt::Request) -> create_new_jwt::Response {
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
