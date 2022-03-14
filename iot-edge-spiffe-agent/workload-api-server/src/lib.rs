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
use log::info;
use node_attestation_agent::NodeAttestation;
use server_agent_api::{create_workload_jwts, get_trust_bundle};
use spiffe_server_client::Client;
use std::{collections::HashMap, sync::Arc};
use tonic::{Request, Response};
use workload_api::{
    spiffe_workload_api_server::SpiffeWorkloadApi, JwtBundlesRequest, JwtBundlesResponse, Jwtsvid,
    JwtsvidRequest, JwtsvidResponse, ValidateJwtsvidRequest, ValidateJwtsvidResponse,
    X509svidRequest, X509svidResponse,
};
use workload_attestation::WorkloadAttestation;

use crate::unix_stream::UdsConnectInfo;

type X509ResponseStream =
    Pin<Box<dyn Stream<Item = Result<X509svidResponse, tonic::Status>> + Send>>;
type JWTResponseStream =
    Pin<Box<dyn Stream<Item = Result<JwtBundlesResponse, tonic::Status>> + Send>>;

pub struct WorkloadAPIServer {
    spiffe_server_client: Arc<dyn Client>,
    workload_attestation: Arc<dyn WorkloadAttestation>,
    node_attestation: Arc<dyn NodeAttestation>,
}

impl WorkloadAPIServer {
    #[must_use]
    pub fn new(
        spiffe_server_client: Arc<dyn Client>,
        workload_attestation: Arc<dyn WorkloadAttestation>,
        node_attestation: Arc<dyn NodeAttestation>,
    ) -> Self {
        Self {
            spiffe_server_client,
            workload_attestation,
            node_attestation,
        }
    }

    async fn fetch_jwtsvid_inner(
        &self,
        request: Request<JwtsvidRequest>,
        pid: u32,
    ) -> Result<Response<JwtsvidResponse>, tonic::Status> {
        let workload_attributes = self
            .workload_attestation
            .attest_workload(pid)
            .await
            .map_err(Error::WorkloadAttestation)?;

        let attestation_token = self
            .node_attestation
            .get_attestation_token()
            .await
            .map_err(Error::NodeAttestation)?;

        let workload_spiffe_id = if request.get_ref().spiffe_id.is_empty() {
            None
        } else {
            Some(request.get_ref().spiffe_id.clone())
        };

        let request = create_workload_jwts::Request {
            workload_spiffe_id,
            audiences: Vec::new(),
            selectors: workload_attributes.selectors,
            attestation_token,
        };

        let svids: Vec<Jwtsvid> = self
            .spiffe_server_client
            .create_workload_jwts(request)
            .await
            .map_err(Error::CreateJWTSVIDs)?
            .jwt_svids
            .into_iter()
            .map(|jwt_svid| Jwtsvid {
                spiffe_id: jwt_svid.spiffe_id.to_string(),
                svid: jwt_svid.token,
            })
            .collect();

        let response = Response::new(JwtsvidResponse { svids });

        Ok(response)
    }
}

#[tonic::async_trait]
impl SpiffeWorkloadApi for WorkloadAPIServer {
    async fn fetch_jwtsvid(
        &self,
        request: Request<JwtsvidRequest>,
    ) -> Result<Response<JwtsvidResponse>, tonic::Status> {
        info!("Received for new jwt");

        let pid = request
            .extensions()
            .get::<UdsConnectInfo>()
            .ok_or(Error::UdsClientPID)?
            .peer_cred
            .ok_or(Error::UdsClientPID)?
            .pid()
            .ok_or(Error::UdsClientPID)?
            .try_into()
            .map_err(Error::NegativePID)?;

        // Create inner to avoid dependency with pid which is very hard to mock
        self.fetch_jwtsvid_inner(request, pid).await
    }

    async fn fetch_jwt_bundles(
        &self,
        _request: Request<JwtBundlesRequest>,
    ) -> Result<Response<Self::FetchJWTBundlesStream>, tonic::Status> {
        info!("Received request for trust bundle");

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
    use crate::WorkloadAPIServer;
    use core_objects::{Crv, JWKSet, JWTSVIDCompact, KeyUse, Kty, TrustBundle, JWK, SPIFFEID};
    use futures_util::StreamExt;
    use node_attestation_agent::MockNodeAttestation;
    use server_agent_api::{create_workload_jwts, get_trust_bundle};
    use spiffe_server_client::MockClient;
    use std::{collections::BTreeSet, io::ErrorKind, sync::Arc};
    use tonic::Request;
    use workload_api::{
        spiffe_workload_api_server::SpiffeWorkloadApi, JwtBundlesRequest, JwtsvidRequest,
    };
    use workload_attestation::{MockWorkloadAttestation, WorkloadAttributes};

    #[tokio::test]
    async fn fetch_jwt_bundles_happy_path() {
        let mut mock_client = MockClient::new();
        let mock_workload_attestation = MockWorkloadAttestation::new();
        let mock_node_attestation = MockNodeAttestation::new();

        let trust_domain = "dummy".to_string();
        let jwk_set = JWKSet {
            keys: vec![JWK {
                x: "xxx".to_string(),
                y: "yyy".to_string(),
                kty: Kty::EC,
                crv: Crv::P256,
                kid: "132".to_string(),
                key_use: KeyUse::JWTSVID,
            }],
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

        let workload_server = WorkloadAPIServer::new(
            Arc::new(mock_client),
            Arc::new(mock_workload_attestation),
            Arc::new(mock_node_attestation),
        );

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
        let mock_workload_attestation = MockWorkloadAttestation::new();
        let mock_node_attestation = MockNodeAttestation::new();

        mock_client.expect_get_trust_bundle().return_once(move |_| {
            // Use full name here to avoid name collision
            Err(Box::new(
                spiffe_server_client::http::error::Error::Connector("dummy".to_string()),
            ))
        });

        let workload_server = WorkloadAPIServer::new(
            Arc::new(mock_client),
            Arc::new(mock_workload_attestation),
            Arc::new(mock_node_attestation),
        );

        let request = Request::new(JwtBundlesRequest::default());
        // Unwrap error doesn't work because the debug trait is missing.
        if workload_server.fetch_jwt_bundles(request).await.is_ok() {
            panic!("Expected an error");
        }
    }

    #[tokio::test]
    async fn fetch_jwtsvid_happy_path() {
        let mut mock_client = MockClient::new();
        let mut mock_workload_attestation = MockWorkloadAttestation::new();
        let mut mock_node_attestation = MockNodeAttestation::new();

        let spiffe_id = SPIFFEID {
            trust_domain: "trust_domain".to_string(),
            path: "path".to_string(),
        };

        let spiffe_id_tmp = spiffe_id.clone();
        mock_client
            .expect_create_workload_jwts()
            .return_once(move |_| {
                Ok(create_workload_jwts::Response {
                    jwt_svids: vec![JWTSVIDCompact {
                        token: "token".to_string(),
                        spiffe_id: spiffe_id_tmp,
                        expiry: 0,
                        issued_at: 0,
                    }],
                })
            });
        mock_workload_attestation
            .expect_attest_workload()
            .return_once(move |_| {
                Ok(WorkloadAttributes {
                    selectors: BTreeSet::new(),
                })
            });

        mock_node_attestation
            .expect_get_attestation_token()
            .return_once(move || Ok("".to_string()));

        let workload_server = WorkloadAPIServer::new(
            Arc::new(mock_client),
            Arc::new(mock_workload_attestation),
            Arc::new(mock_node_attestation),
        );
        let request = Request::new(JwtsvidRequest::default());

        let response = workload_server
            .fetch_jwtsvid_inner(request, 0)
            .await
            .unwrap()
            .into_inner();

        let resp = response.svids;
        let jwt_svid = resp.first().unwrap();

        assert_eq!(1, resp.len());
        assert_eq!(spiffe_id.to_string(), jwt_svid.spiffe_id.to_string());
        assert_eq!("token", jwt_svid.svid);
    }

    #[tokio::test]
    async fn fetch_jwtsvid_error_workload_attestation() {
        let mut mock_client = MockClient::new();
        let mock_workload_attestation = MockWorkloadAttestation::new();
        let mock_node_attestation = MockNodeAttestation::new();

        mock_client
            .expect_create_workload_jwts()
            .return_once(move |_| {
                // Use full name here to avoid name collision
                Err(Box::new(
                    spiffe_server_client::http::error::Error::Connector("dummy".to_string()),
                ))
            });

        let workload_server = WorkloadAPIServer::new(
            Arc::new(mock_client),
            Arc::new(mock_workload_attestation),
            Arc::new(mock_node_attestation),
        );

        let request = Request::new(JwtsvidRequest::default());
        // Unwrap error doesn't work because the debug trait is missing.
        if workload_server.fetch_jwtsvid(request).await.is_ok() {
            panic!("Expected an error");
        }
    }

    #[tokio::test]
    async fn fetch_jwtsvid_error_agent_attestation() {
        let mut mock_client = MockClient::new();
        let mut mock_workload_attestation = MockWorkloadAttestation::new();
        let mut mock_node_attestation = MockNodeAttestation::new();

        let spiffe_id = SPIFFEID {
            trust_domain: "trust_domain".to_string(),
            path: "path".to_string(),
        };

        let spiffe_id_tmp = spiffe_id.clone();
        mock_client
            .expect_create_workload_jwts()
            .return_once(move |_| {
                Ok(create_workload_jwts::Response {
                    jwt_svids: vec![JWTSVIDCompact {
                        token: "token".to_string(),
                        spiffe_id: spiffe_id_tmp,
                        expiry: 0,
                        issued_at: 0,
                    }],
                })
            });
        mock_workload_attestation
            .expect_attest_workload()
            .return_once(move |_| {
                Err(Box::new(
                    node_attestation_agent::k8s::error::Error::UnableToReadToken(
                        std::io::Error::new(ErrorKind::Other, "dummy"),
                    ),
                ))
            });

        mock_node_attestation
            .expect_get_attestation_token()
            .return_once(move || Ok("".to_string()));

        let workload_server = WorkloadAPIServer::new(
            Arc::new(mock_client),
            Arc::new(mock_workload_attestation),
            Arc::new(mock_node_attestation),
        );
        let request = Request::new(JwtsvidRequest::default());
        // Unwrap error doesn't work because the debug trait is missing.
        if workload_server.fetch_jwtsvid(request).await.is_ok() {
            panic!("Expected an error");
        }
    }
}
