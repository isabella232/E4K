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
use jwt_svid_validator::JWTSVIDValidator;
use log::{debug, info};
use node_attestation_agent::NodeAttestation;
use server_agent_api::{create_workload_jwts, get_trust_bundle};
use spiffe_server_client::Client;
use std::{collections::HashMap, sync::Arc};
use tonic::{Request, Response};
use trust_bundle_manager::TrustBundleManager;
use workload_api::generated::{
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
    trust_bundle_manager: Arc<TrustBundleManager>,
    jwt_svid_validator: Arc<dyn JWTSVIDValidator>,
}

impl WorkloadAPIServer {
    #[must_use]
    pub fn new(
        spiffe_server_client: Arc<dyn Client>,
        workload_attestation: Arc<dyn WorkloadAttestation>,
        node_attestation: Arc<dyn NodeAttestation>,
        trust_bundle_manager: Arc<TrustBundleManager>,
        jwt_svid_validator: Arc<dyn JWTSVIDValidator>,
    ) -> Self {
        Self {
            spiffe_server_client,
            workload_attestation,
            node_attestation,
            trust_bundle_manager,
            jwt_svid_validator,
        }
    }

    async fn fetch_jwtsvid_inner(
        &self,
        request: Request<JwtsvidRequest>,
        pid: u32,
    ) -> Result<Response<JwtsvidResponse>, tonic::Status> {
        let jwt_svid_request = request.into_inner();
        debug!("Request: {:?}", jwt_svid_request);

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

        let workload_spiffe_id = if jwt_svid_request.spiffe_id.is_empty() {
            None
        } else {
            Some(jwt_svid_request.spiffe_id.clone())
        };

        let request = create_workload_jwts::Request {
            workload_spiffe_id,
            audiences: jwt_svid_request.audience,
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
        request: Request<ValidateJwtsvidRequest>,
    ) -> Result<Response<ValidateJwtsvidResponse>, tonic::Status> {
        let request = request.into_inner();

        info!("Received request for to validate jwt svid");
        debug!("SVID: {:?}, Audience: {}", request.svid, request.audience);
        let trust_bundle = self.trust_bundle_manager.get_cached_trust_bundle().await;

        let audience = request.audience;
        let jwt_svid_compact = request.svid;

        let jwt_svid = self
            .jwt_svid_validator
            .validate(&jwt_svid_compact, &trust_bundle, &audience)
            .await
            .map_err(Error::ValidateJWTSVIDs)?;

        let claims_struct =
            serde_json::from_str(&serde_json::to_string(&jwt_svid.claims).unwrap()).unwrap();

        Ok(Response::new(ValidateJwtsvidResponse {
            spiffe_id: jwt_svid.claims.subject,
            claims: Some(claims_struct),
        }))
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
    use core_objects::{
        Crv, JWKSet, JWTClaims, JWTHeader, JWTSVIDCompact, JWTType, KeyType, KeyUse, Kty,
        TrustBundle, JWK, JWTSVID,
    };
    use futures_util::StreamExt;
    use jwt_svid_validator::MockJWTSVIDValidator;
    use node_attestation_agent::MockNodeAttestation;
    use server_agent_api::{create_workload_jwts, get_trust_bundle};
    use spiffe_server_client::MockClient;
    use std::{collections::BTreeSet, io::ErrorKind, sync::Arc};
    use tonic::Request;
    use trust_bundle_manager::TrustBundleManager;
    use workload_api::generated::{
        spiffe_workload_api_server::SpiffeWorkloadApi, JwtBundlesRequest, JwtsvidRequest,
        ValidateJwtsvidRequest,
    };
    use workload_attestation::{MockWorkloadAttestation, WorkloadAttributes};

    fn init() -> (
        MockClient,
        MockWorkloadAttestation,
        MockNodeAttestation,
        MockJWTSVIDValidator,
        TrustBundle,
    ) {
        let mock_client = MockClient::new();
        let mock_workload_attestation = MockWorkloadAttestation::new();
        let mock_node_attestation = MockNodeAttestation::new();
        let mock_jwt_svid_validator = MockJWTSVIDValidator::new();

        let jwk = JWK {
            x: "MjE2NDE3NTMwMTgxMjY5Njc2MTE3MzAwODU4NjY4Mjg2MDU4MTQ2OTY3ODY0MjU2MDA1MzI0NTA0ODQyNTcxMTcyMzI4NjM1MjgxMjM".to_string(),
            y: "MzU1NjA3MjI0Mjc5MzAxMjYzMzkxNDg5NjAxMDA2NjMzNDE1NTA2MzQzMTQ5MDIxNzQxNTI0MDMyMzk0ODA1NjM2NjE0MTU0NjMyNzI".to_string(),
            kty: Kty::EC,
            crv: Crv::P256,
            kid: "kid".to_string(),
            key_use: KeyUse::JWTSVID,
        };

        let trust_bundle = TrustBundle {
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
        };

        (
            mock_client,
            mock_workload_attestation,
            mock_node_attestation,
            mock_jwt_svid_validator,
            trust_bundle,
        )
    }

    #[tokio::test]
    async fn validate_jwt_happy_path() {
        let (
            mock_client,
            mock_workload_attestation,
            mock_node_attestation,
            mut mock_jwt_svid_validator,
            trust_bundle,
        ) = init();

        let request = Request::new(ValidateJwtsvidRequest::default());

        let mock_client = Arc::new(mock_client);
        let trust_bundle_manager = TrustBundleManager::new(mock_client.clone(), trust_bundle);

        let header = JWTHeader {
            algorithm: KeyType::ES256,
            key_id: "kid".to_string(),
            jwt_type: JWTType::JOSE,
        };

        let claims = JWTClaims {
            subject: "subject".to_string(),
            audience: vec!["audience".to_string()],
            expiry: 10,
            issued_at: 0,
            other_identities: Vec::new(),
        };
        mock_jwt_svid_validator.expect_validate().return_once({
            let claims = claims.clone();

            move |_, _, _| {
                Ok(JWTSVID {
                    header,
                    claims,
                    signature: "dummy".to_string(),
                })
            }
        });

        let workload_server = WorkloadAPIServer::new(
            mock_client,
            Arc::new(mock_workload_attestation),
            Arc::new(mock_node_attestation),
            Arc::new(trust_bundle_manager),
            Arc::new(mock_jwt_svid_validator),
        );

        let response = workload_server
            .validate_jwtsvid(request)
            .await
            .unwrap()
            .into_inner();
        assert_eq!(response.spiffe_id, "subject");

        let res_claims = response.claims.unwrap();

        assert_eq!(
            res_claims,
            serde_json::from_str(&serde_json::to_string(&claims).unwrap()).unwrap()
        );
    }

    #[tokio::test]
    #[allow(clippy::cast_precision_loss)]
    async fn validate_jwt_error_validation() {
        let (
            mock_client,
            mock_workload_attestation,
            mock_node_attestation,
            mut mock_jwt_svid_validator,
            trust_bundle,
        ) = init();

        let request = Request::new(ValidateJwtsvidRequest::default());

        let mock_client = Arc::new(mock_client);
        let trust_bundle_manager = TrustBundleManager::new(mock_client.clone(), trust_bundle);
        mock_jwt_svid_validator
            .expect_validate()
            .return_once(move |_, _, _| Err(jwt_svid_validator::error::Error::InvalidSignature));

        let workload_server = WorkloadAPIServer::new(
            mock_client,
            Arc::new(mock_workload_attestation),
            Arc::new(mock_node_attestation),
            Arc::new(trust_bundle_manager),
            Arc::new(mock_jwt_svid_validator),
        );

        // Unwrap error doesn't work because the debug trait is missing.
        assert!(
            workload_server.validate_jwtsvid(request).await.is_err(),
            "Expected an error"
        );
    }

    #[tokio::test]
    async fn fetch_jwt_bundles_happy_path() {
        let (
            mut mock_client,
            mock_workload_attestation,
            mock_node_attestation,
            mock_jwt_svid_validator,
            trust_bundle,
        ) = init();

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

        let mock_client = Arc::new(mock_client);
        let trust_bundle_manager = TrustBundleManager::new(mock_client.clone(), trust_bundle);

        let workload_server = WorkloadAPIServer::new(
            mock_client,
            Arc::new(mock_workload_attestation),
            Arc::new(mock_node_attestation),
            Arc::new(trust_bundle_manager),
            Arc::new(mock_jwt_svid_validator),
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
        let (
            mut mock_client,
            mock_workload_attestation,
            mock_node_attestation,
            mock_jwt_svid_validator,
            trust_bundle,
        ) = init();

        mock_client.expect_get_trust_bundle().return_once(move |_| {
            // Use full name here to avoid name collision
            Err(Box::new(
                spiffe_server_client::http::error::Error::Connector("dummy".to_string()),
            ))
        });

        let mock_client = Arc::new(mock_client);
        let trust_bundle_manager = TrustBundleManager::new(mock_client.clone(), trust_bundle);

        let workload_server = WorkloadAPIServer::new(
            mock_client,
            Arc::new(mock_workload_attestation),
            Arc::new(mock_node_attestation),
            Arc::new(trust_bundle_manager),
            Arc::new(mock_jwt_svid_validator),
        );

        let request = Request::new(JwtBundlesRequest::default());
        // Unwrap error doesn't work because the debug trait is missing.
        assert!(
            workload_server.fetch_jwt_bundles(request).await.is_err(),
            "Expected an error"
        );
    }

    #[tokio::test]
    async fn fetch_jwtsvid_happy_path() {
        let (
            mut mock_client,
            mut mock_workload_attestation,
            mut mock_node_attestation,
            mock_jwt_svid_validator,
            trust_bundle,
        ) = init();

        let spiffe_id = "trust_domain/path".to_string();

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

        let mock_client = Arc::new(mock_client);
        let trust_bundle_manager = TrustBundleManager::new(mock_client.clone(), trust_bundle);

        let workload_server = WorkloadAPIServer::new(
            mock_client,
            Arc::new(mock_workload_attestation),
            Arc::new(mock_node_attestation),
            Arc::new(trust_bundle_manager),
            Arc::new(mock_jwt_svid_validator),
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
        let (
            mut mock_client,
            mock_workload_attestation,
            mock_node_attestation,
            mock_jwt_svid_validator,
            trust_bundle,
        ) = init();

        mock_client
            .expect_create_workload_jwts()
            .return_once(move |_| {
                // Use full name here to avoid name collision
                Err(Box::new(
                    spiffe_server_client::http::error::Error::Connector("dummy".to_string()),
                ))
            });

        let mock_client = Arc::new(mock_client);
        let trust_bundle_manager = TrustBundleManager::new(mock_client.clone(), trust_bundle);

        let workload_server = WorkloadAPIServer::new(
            mock_client,
            Arc::new(mock_workload_attestation),
            Arc::new(mock_node_attestation),
            Arc::new(trust_bundle_manager),
            Arc::new(mock_jwt_svid_validator),
        );

        let request = Request::new(JwtsvidRequest::default());
        // Unwrap error doesn't work because the debug trait is missing.
        assert!(
            workload_server.fetch_jwtsvid(request).await.is_err(),
            "Expected an error"
        );
    }

    #[tokio::test]
    async fn fetch_jwtsvid_error_agent_attestation() {
        let (
            mut mock_client,
            mut mock_workload_attestation,
            mut mock_node_attestation,
            mock_jwt_svid_validator,
            trust_bundle,
        ) = init();

        let spiffe_id = "trust_domain/path".to_string();

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

        let mock_client = Arc::new(mock_client);
        let trust_bundle_manager = TrustBundleManager::new(mock_client.clone(), trust_bundle);

        let workload_server = WorkloadAPIServer::new(
            mock_client,
            Arc::new(mock_workload_attestation),
            Arc::new(mock_node_attestation),
            Arc::new(trust_bundle_manager),
            Arc::new(mock_jwt_svid_validator),
        );
        let request = Request::new(JwtsvidRequest::default());
        // Unwrap error doesn't work because the debug trait is missing.
        assert!(
            workload_server.fetch_jwtsvid(request).await.is_err(),
            "Expected an error"
        );
    }
}
