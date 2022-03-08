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

mod error;
pub mod unix_stream;

use core::pin::Pin;
use error::Error;
use futures_util::Stream;
use server_agent_api::get_trust_bundle;
use spiffe_server_client::Client;
use std::{collections::HashMap, sync::Arc};
use tonic::{Request, Response};
use workload_api::{
    spiffe_workload_api_server::SpiffeWorkloadApi, JwtBundlesRequest, JwtBundlesResponse,
    JwtsvidRequest, JwtsvidResponse, ValidateJwtsvidRequest, ValidateJwtsvidResponse,
    X509svidRequest, X509svidResponse,
};

type X509ResponseStream =
    Pin<Box<dyn Stream<Item = Result<X509svidResponse, tonic::Status>> + Send>>;
type JWTResponseStream =
    Pin<Box<dyn Stream<Item = Result<JwtBundlesResponse, tonic::Status>> + Send>>;

pub struct WorkloadAPIServer {
    spiffe_server_client: Arc<dyn Client + Sync + Send>,
}

impl WorkloadAPIServer {
    #[must_use]
    pub fn new(spiffe_server_client: Arc<dyn Client + Sync + Send>) -> Self {
        Self {
            spiffe_server_client,
        }
    }
}

#[tonic::async_trait]
impl SpiffeWorkloadApi for WorkloadAPIServer {
    async fn fetch_jwtsvid(
        &self,
        _request: Request<JwtsvidRequest>,
    ) -> Result<Response<JwtsvidResponse>, tonic::Status> {
        todo!()
    }

    async fn fetch_jwt_bundles(
        &self,
        _request: Request<JwtBundlesRequest>,
    ) -> Result<Response<Self::FetchJWTBundlesStream>, tonic::Status> {
        let mut bundles_map = HashMap::new();

        let trust_bundle = self
            .spiffe_server_client
            .get_trust_bundle(get_trust_bundle::Params {
                jwt_keys: true,
                x509_cas: false,
            })
            .await
            .map_err(Error::TrustBundleResponse)?
            .trust_bundle;

        let jwk_set =
            serde_json::to_vec(&trust_bundle.jwt_key_set).map_err(Error::SerdeConvertToVec)?;

        bundles_map.insert(trust_bundle.trust_domain, jwk_set);

        let trust_bundle_response = JwtBundlesResponse {
            bundles: bundles_map,
        };

        let stream: Self::FetchJWTBundlesStream = Box::pin(async_stream::stream! {
                yield Ok(trust_bundle_response)
        }) as _;

        return Ok(Response::new(Box::pin(stream) as _));
    }

    async fn validate_jwtsvid(
        &self,
        _request: Request<ValidateJwtsvidRequest>,
    ) -> Result<Response<ValidateJwtsvidResponse>, tonic::Status> {
        todo!()
    }

    type FetchX509SVIDStream = X509ResponseStream;

    async fn fetch_x509svid(
        &self,
        _request: Request<X509svidRequest>,
    ) -> Result<Response<Self::FetchX509SVIDStream>, tonic::Status> {
        todo!()
    }

    type FetchJWTBundlesStream = JWTResponseStream;
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::WorkloadAPIServer;
    use core_objects::{Crv, JWKSet, KeyUse, Kty, TrustBundle, JWK};
    use futures_util::StreamExt;
    use server_agent_api::get_trust_bundle;
    use spiffe_server_client::MockClient;
    use tonic::Request;
    use workload_api::{spiffe_workload_api_server::SpiffeWorkloadApi, JwtBundlesRequest};

    #[tokio::test]
    async fn fetch_jwt_bundles_happy_path() {
        let mut mock_client = MockClient::new();

        let trust_domain = "dummy".to_string();
        let jwk_set = JWKSet {
            keys: [JWK {
                x: "xxx".to_string(),
                y: "yyy".to_string(),
                kty: Kty::EC,
                crv: Crv::P256,
                kid: "132".to_string(),
                key_use: KeyUse::JWTSVID,
            }]
            .to_vec(),
            spiffe_refresh_hint: 0,
            spiffe_sequence_number: 0,
        };

        let closure_jwk_set = jwk_set.clone();
        mock_client.expect_get_trust_bundle().return_once(move |_| {
            Ok(get_trust_bundle::Response {
                trust_bundle: TrustBundle {
                    trust_domain: trust_domain.to_string(),
                    jwt_key_set: closure_jwk_set,
                    x509_key_set: JWKSet {
                        keys: Vec::new(),
                        spiffe_refresh_hint: 0,
                        spiffe_sequence_number: 0,
                    },
                },
            })
        });
        let workload_server = WorkloadAPIServer::new(Arc::new(mock_client));

        let request = Request::new(JwtBundlesRequest::default());
        let mut stream = workload_server
            .fetch_jwt_bundles(request)
            .await
            .unwrap()
            .into_inner();
        let (trust_domain_resp, jwk_set_resp) = stream
            .next()
            .await
            .unwrap()
            .unwrap()
            .bundles
            .into_iter()
            .last()
            .unwrap();
        let jwk_set_resp: JWKSet = serde_json::from_slice(&jwk_set_resp).unwrap();

        assert_eq!(trust_domain_resp, "dummy");
        assert_eq!(jwk_set_resp, jwk_set);
    }

    #[tokio::test]
    async fn fetch_jwt_bundles_no_server_response() {
        let mut mock_client = MockClient::new();
        mock_client.expect_get_trust_bundle().return_once(move |_| {
            // Use full name here to avoid name collision
            Err(Box::new(
                spiffe_server_client::http::error::Error::Connector("dummy".to_string()),
            ))
        });

        let workload_server = WorkloadAPIServer::new(Arc::new(mock_client));

        let request = Request::new(JwtBundlesRequest::default());
        // Unwrap error doesn't work because the debug trait is missing.
        if workload_server.fetch_jwt_bundles(request).await.is_ok() {
            panic!("Expected an error");
        }
    }
}
