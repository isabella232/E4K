// Copyright (c) Microsoft. All rights reserved.

pub mod error;

use std::{fs, path, sync::Arc};

use agent_config::NodeAttestationConfigK8s;
use core_objects::JWTSVIDCompact;
use server_agent_api::attest_agent;
use server_client::Client;

use crate::NodeAttestation as NodeAttestationTrait;

use error::Error;

pub struct NodeAttestation {
    token_path: path::PathBuf,
    server_api_client: Arc<dyn Client + Sync + Send>,
}

impl NodeAttestation {
    #[must_use]
    pub fn new(
        config: &NodeAttestationConfigK8s,
        server_api_client: Arc<dyn Client + Sync + Send>,
    ) -> Self {
        let token_path = path::Path::new(&config.token_path).to_path_buf();
        NodeAttestation {
            token_path,
            server_api_client,
        }
    }
}

#[async_trait::async_trait]
impl NodeAttestationTrait for NodeAttestation {
    async fn attest_agent(&self) -> Result<JWTSVIDCompact, Box<dyn std::error::Error + Send>> {
        let token = fs::read_to_string(&self.token_path)
            .map_err(|err| Box::new(Error::UnableToReadToken(err)) as _)?;
        let auth = attest_agent::Auth { token };
        let response = self.server_api_client.attest_agent(auth).await?;

        Ok(response.jwt_svid)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::Arc;

    use crate::k8s::Error;
    use crate::k8s::NodeAttestation;
    use crate::NodeAttestation as NodeAttestationTrait;
    use agent_config::Config;
    use agent_config::NodeAttestationConfig::Psat;
    use core_objects::{JWTSVIDCompact, AGENT_DEFAULT_CONFIG_PATH, SPIFFEID};
    use matches::assert_matches;
    use server_agent_api::attest_agent;
    use server_client::MockClient;
    use tempdir::TempDir;

    fn init_tests() -> (Config, String) {
        let dir = TempDir::new("test").unwrap();
        let base_path = dir.into_path().to_str().unwrap().to_string();

        let config = Config::load_config(AGENT_DEFAULT_CONFIG_PATH).unwrap();

        (config, base_path)
    }

    #[tokio::test]
    async fn attest_agent_happy_path() {
        let (mut config, base_path) = init_tests();

        let token_path = format!("{}/{}", base_path, "psat_token");
        fs::write(token_path.clone(), "dummy token").unwrap();

        let config = if let Psat(config) = &mut config.node_attestation_config {
            config
        } else {
            panic!("Unexpected attestation type");
        };

        config.token_path = token_path;

        let jwt_svid = JWTSVIDCompact {
            token: "dummy".to_string(),
            spiffe_id: SPIFFEID {
                trust_domain: "dummy".to_string(),
                path: "dummy".to_string(),
            },
            expiry: 0,
            issued_at: 0,
        };
        let jwt_copy = jwt_svid.clone();
        let mut mock = MockClient::new();
        mock.expect_attest_agent().returning(move |_| {
            Ok(attest_agent::Response {
                jwt_svid: jwt_copy.clone(),
            })
        });

        let node_attestation = NodeAttestation::new(config, Arc::new(mock));

        let resp = node_attestation.attest_agent().await.unwrap();

        assert_eq!(resp, jwt_svid);
    }

    #[tokio::test]
    async fn attest_agent_read_token_error() {
        let (mut config, _base_path) = init_tests();

        let config = if let Psat(config) = &mut config.node_attestation_config {
            config
        } else {
            panic!("Unexpected attestation type");
        };
        let mock = MockClient::new();

        let node_attestation = NodeAttestation::new(config, Arc::new(mock));

        let error = *node_attestation
            .attest_agent()
            .await
            .unwrap_err()
            .downcast::<Error>()
            .unwrap();

        assert_matches!(error, Error::UnableToReadToken(_));
    }
}
