// Copyright (c) Microsoft. All rights reserved.

pub mod error;

use std::{fs, path};

use agent_config::NodeAttestationConfigK8s;

use crate::NodeAttestation as NodeAttestationTrait;

use error::Error;

pub struct NodeAttestation {
    token_path: path::PathBuf,
}

impl NodeAttestation {
    #[must_use]
    pub fn new(config: &NodeAttestationConfigK8s) -> Self {
        let token_path = path::Path::new(&config.token_path).to_path_buf();
        NodeAttestation { token_path }
    }
}

#[async_trait::async_trait]
impl NodeAttestationTrait for NodeAttestation {
    async fn get_attestation_token(&self) -> Result<String, Box<dyn std::error::Error + Send>> {
        let token = fs::read_to_string(&self.token_path)
            .map_err(|err| Box::new(Error::UnableToReadToken(err)) as _)?;

        Ok(token)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::k8s::Error;
    use crate::k8s::NodeAttestation;
    use crate::NodeAttestation as NodeAttestationTrait;
    use agent_config::Config;
    use agent_config::NodeAttestationConfig::Psat;
    use core_objects::AGENT_DEFAULT_CONFIG_PATH;
    use matches::assert_matches;
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

        let node_attestation = NodeAttestation::new(config);

        let token = node_attestation.get_attestation_token().await.unwrap();

        assert_eq!(token, "dummy token");
    }

    #[tokio::test]
    async fn attest_agent_read_token_error() {
        let (mut config, _base_path) = init_tests();

        let config = if let Psat(config) = &mut config.node_attestation_config {
            config
        } else {
            panic!("Unexpected attestation type");
        };

        let node_attestation = NodeAttestation::new(config);

        let error = *node_attestation
            .get_attestation_token()
            .await
            .unwrap_err()
            .downcast::<Error>()
            .unwrap();

        assert_matches!(error, Error::UnableToReadToken(_));
    }
}
