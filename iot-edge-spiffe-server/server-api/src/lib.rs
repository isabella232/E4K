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

use http_common::Connector;
use identity_matcher::IdentityMatcher;
use node_attestation_server::NodeAttestation;
use server_config::Config;
use std::{io, sync::Arc};
use svid_factory::SVIDFactory;
use tokio::task::JoinHandle;
use trust_bundle_builder::TrustBundleBuilder;

pub mod create_workload_jwts;
mod error;
mod http;

const SOCKET_DEFAULT_PERMISSION: u32 = 0o660;

pub async fn start_server_api(
    config: &Config,
    svid_factory: Arc<SVIDFactory>,
    trust_bundle_builder: Arc<TrustBundleBuilder>,
    node_attestation: Arc<dyn NodeAttestation>,
    identity_matcher: Arc<IdentityMatcher>,
) -> Result<JoinHandle<Result<(), std::io::Error>>, io::Error> {
    let api = Api {
        svid_factory,
        trust_bundle_builder,
        node_attestation,
        identity_matcher,
    };

    let service = http::Service { api };
    let uri: &str = &config.server_agent_api.bind_address;

    let connector = Connector::Tcp {
        host: uri.into(),
        port: config.server_agent_api.bind_port,
    };

    let mut incoming = connector.incoming(SOCKET_DEFAULT_PERMISSION, None).await?;

    Ok(tokio::spawn(async move {
        // Channel to gracefully shut down the server. It's currently not used.
        let (_shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();

        log::info!("Starting SVID & trust bundle server");
        let res = incoming.serve(service, shutdown_rx).await;
        if let Err(err) = res {
            log::error!("Closing SVID & trust bundle server: {:?}", err);
        } else {
            log::info!("Closing SVID & trust bundle server");
        };

        Ok(())
    }))
}

#[derive(Clone)]
struct Api {
    svid_factory: Arc<SVIDFactory>,
    trust_bundle_builder: Arc<TrustBundleBuilder>,
    node_attestation: Arc<dyn NodeAttestation>,
    identity_matcher: Arc<IdentityMatcher>,
}
