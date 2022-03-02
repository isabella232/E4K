// Copyright (c) Microsoft. All rights reserved.

use crate::{error::Error, Api};
use server_agent_api::attest_agent;

impl Api {
    pub async fn attest_agent(&self, token: &str) -> Result<attest_agent::Response, Error> {
        let agent_attributes = self
            .node_attestation
            .attest_agent(token)
            .await
            .map_err(Error::AttestAgent)?;

        self.catalog
            .set_selectors(&agent_attributes.spiffe_id, agent_attributes.selectors)
            .await
            .map_err(Error::CatalogSetSelectors)?;

        let jwt_svid_params = svid_factory::JWTSVIDParams {
            spiffe_id: agent_attributes.spiffe_id,
            audiences: [self.iotedge_server_spiffe_id.clone()].to_vec(),
            other_identities: Vec::new(),
        };
        let jwt_svid = self
            .svid_factory
            .create_jwt_svid(jwt_svid_params)
            .await
            .map_err(Error::CreateAgentJWT)?;

        Ok(attest_agent::Response { jwt_svid })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use catalog::{inmemory, NodeSelectors};
    use core_objects::{CONFIG_DEFAULT_PATH, SPIFFEID};
    use key_manager::KeyManager;
    use key_store::disk;
    use matches::assert_matches;
    use mock_kube::{get_nodes, get_pods, get_token_review, Client};
    use node_attestation_server::NodeAttestatorFactory;
    use server_config::{Config, KeyStoreConfig, KeyStoreConfigDisk};
    use svid_factory::SVIDFactory;
    use trust_bundle_builder::TrustBundleBuilder;

    use std::sync::Arc;
    use tempdir::TempDir;

    async fn init(error_injection: bool) -> (Api, Arc<inmemory::Catalog>) {
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

        let trust_bundle_builder = TrustBundleBuilder::new(&config, catalog.clone());
        let svid_factory = Arc::new(SVIDFactory::new(key_manager.clone(), &config));
        let iotedge_server_spiffe_id = SPIFFEID {
            trust_domain: "trust_domain".to_string(),
            path: "path".to_string(),
        };

        let mut client = Client::try_default().await.unwrap();

        let pod = get_pods();
        let node = get_nodes();
        let mut token_review = get_token_review();
        // We inject an error to test the failure flag
        if error_injection {
            token_review.status = None;
        }

        client.queue_response(token_review).await;
        client.queue_response(pod).await;
        client.queue_response(node).await;

        let node_attestation = NodeAttestatorFactory::get(
            &config.node_attestation_config,
            &config.trust_domain,
            client,
        );

        let api = Api {
            catalog: catalog.clone(),
            svid_factory,
            trust_bundle_builder,
            node_attestation,
            iotedge_server_spiffe_id,
        };

        (api, catalog)
    }

    #[tokio::test]
    async fn attest_agent_happy_path() {
        let (api, catalog) = init(false).await;

        let resp = api.attest_agent("dummytoken").await.unwrap();

        assert_eq!(&resp.jwt_svid.spiffe_id.to_string(), "iotedge/iotedge/spiffe-agent/k8s-psat/demo-cluster/14b57414-9516-11ec-b909-0242ac120002");

        // Check selectors have been put in the catalog
        catalog
            .get_selectors(&resp.jwt_svid.spiffe_id)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn attest_agent_attest_agent_error_path() {
        let (api, _catalog) = init(true).await;

        let error = api.attest_agent("dummytoken").await.unwrap_err();
        assert_matches!(error, Error::AttestAgent(_));
    }
}
