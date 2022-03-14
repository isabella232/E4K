// Copyright (c) Microsoft. All rights reserved.

use server_agent_api::{create_workload_jwts, get_trust_bundle};
use svid_factory::JWTSVIDParams;

use crate::{error::Error, Api};

impl Api {
    pub async fn create_workload_jwts(
        &self,
        req: create_workload_jwts::Request,
    ) -> Result<create_workload_jwts::Response, Error> {
        let agent_attributes = self
            .node_attestation
            .attest_agent(&req.attestation_token)
            .await
            .map_err(Error::AttestAgent)?;

        let entries = self
            .identity_matcher
            .get_entry_id_from_selectors(&req.selectors, &agent_attributes.selectors)
            .await
            .map_err(Error::MatchIdentity)?;

        let mut jwt_svids = Vec::new();

        for entry in entries {
            let jwt_svid_params = JWTSVIDParams {
                spiffe_id: entry.spiffe_id,
                audiences: req.audiences.clone(),
                other_identities: entry.other_identities,
            };

            let jwt_svid = self
                .svid_factory
                .create_jwt_svid(jwt_svid_params)
                .await
                .map_err(Error::CreateWorkloadJWT)?;

            jwt_svids.push(jwt_svid);
        }

        Ok(create_workload_jwts::Response { jwt_svids })
    }

    pub async fn get_trust_bundle(
        &self,
        params: get_trust_bundle::Params,
    ) -> Result<get_trust_bundle::Response, Error> {
        let trust_bundle = self
            .trust_bundle_builder
            .build_trust_bundle(params.jwt_keys, params.x509_cas)
            .await
            .map_err(Error::BuildTrustBundle)?;

        Ok(get_trust_bundle::Response { trust_bundle })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use catalog::{inmemory, Catalog, Entries};
    use core_objects::{
        AttestationConfig, EntryNodeAttestation, EntryWorkloadAttestation, NodeAttestationPlugin,
        RegistrationEntry, WorkloadAttestationPlugin, CONFIG_DEFAULT_PATH, SPIFFEID,
    };
    use identity_matcher::IdentityMatcher;
    use key_manager::KeyManager;
    use key_store::disk;
    use matches::assert_matches;
    use mock_kube::{get_nodes, get_pods, get_token_review, Client};
    use node_attestation_server::NodeAttestatorFactory;
    use server_config::{Config, KeyStoreConfig, KeyStoreConfigDisk};
    use svid_factory::SVIDFactory;
    use trust_bundle_builder::TrustBundleBuilder;

    use std::{collections::BTreeSet, sync::Arc};
    use tempdir::TempDir;

    async fn init() -> (
        Api,
        Vec<RegistrationEntry>,
        Arc<KeyManager>,
        Config,
        Client,
        Arc<dyn Catalog>,
    ) {
        let mut config = Config::load_config(CONFIG_DEFAULT_PATH).unwrap();
        let dir = TempDir::new("test").unwrap();
        let key_base_path = dir.into_path().to_str().unwrap().to_string();
        let key_plugin = KeyStoreConfigDisk {
            key_base_path: key_base_path.clone(),
        };

        let spiffe_id_parent = SPIFFEID {
            trust_domain: "trust_domain".to_string(),
            path: "parent".to_string(),
        };

        let spiffe_id_generic_workload = SPIFFEID {
            trust_domain: "trust_domain".to_string(),
            path: "generic".to_string(),
        };

        // Create parent
        let entry1 = RegistrationEntry {
            id: String::from("parent"),
            other_identities: Vec::new(),
            spiffe_id: spiffe_id_parent,
            attestation_config: AttestationConfig::Node(EntryNodeAttestation {
                value: vec!["AGENTSERVICEACCOUNT:iotedge-spiffe-agent".to_string()],
                plugin: NodeAttestationPlugin::Psat,
            }),
            admin: false,
            ttl: 0,
            expires_at: 0,
            dns_names: Vec::new(),
            revision_number: 0,
            store_svid: false,
        };

        // Create child
        let entry2 = RegistrationEntry {
            id: String::from("workload"),
            other_identities: Vec::new(),
            spiffe_id: spiffe_id_generic_workload,
            attestation_config: AttestationConfig::Workload(EntryWorkloadAttestation {
                value: vec!["PODLABELS:app:genericnode".to_string()],
                plugin: WorkloadAttestationPlugin::K8s,
                parent_id: "parent".to_string(),
            }),
            admin: false,
            ttl: 0,
            expires_at: 0,
            dns_names: Vec::new(),
            revision_number: 0,
            store_svid: false,
        };
        let entries = vec![entry1, entry2];

        // Change key disk plugin path to write in tempdir
        config.key_store = KeyStoreConfig::Disk(key_plugin.clone());
        // Force ttl to 300s
        config.jwt.key_ttl = 300;

        let catalog = Arc::new(inmemory::Catalog::new());
        catalog.batch_create(entries.clone()).await.unwrap();

        let key_store = Arc::new(disk::KeyStore::new(&key_plugin));

        let key_manager = Arc::new(
            KeyManager::new(&config, catalog.clone(), key_store.clone(), 0)
                .await
                .unwrap(),
        );

        let trust_bundle_builder = TrustBundleBuilder::new(&config, catalog.clone());
        let svid_factory = Arc::new(SVIDFactory::new(key_manager.clone(), &config));

        let client = Client::try_default().await.unwrap();
        let node_attestation =
            NodeAttestatorFactory::get(&config.node_attestation_config, client.clone());
        let identity_matcher = Arc::new(IdentityMatcher::new(catalog.clone()));

        let api = Api {
            svid_factory,
            trust_bundle_builder,
            node_attestation,
            identity_matcher,
        };

        (api, entries, key_manager, config, client, catalog)
    }

    #[tokio::test]
    async fn create_new_jwts_happy_path() {
        let (api, entries, _key_manager, _config, mut client, _catalog) = init().await;

        let entry = entries[1].clone();

        let mut workload_selectors = BTreeSet::new();
        workload_selectors.insert("PODLABELS:app:genericnode".to_string());

        let req = create_workload_jwts::Request {
            audiences: vec![SPIFFEID {
                trust_domain: "my trust domain".to_string(),
                path: "audiences".to_string(),
            }],
            selectors: workload_selectors,
            attestation_token: "dummy".to_string(),
            workload_spiffe_id: None,
        };

        let pod = get_pods();
        let node = get_nodes();
        let token_review = get_token_review();

        client.queue_response(token_review).await;
        client.queue_response(pod).await;
        client.queue_response(node).await;

        let response = api.create_workload_jwts(req).await.unwrap();
        assert_eq!(response.jwt_svids.len(), 1);
        assert_eq!(
            response.jwt_svids[0].spiffe_id.trust_domain,
            entry.spiffe_id.trust_domain
        );
        assert_eq!(response.jwt_svids[0].spiffe_id.path, entry.spiffe_id.path);
    }

    #[tokio::test]
    async fn create_new_jwts_attest_agent_error() {
        let (api, _entries, _key_manager, _config, mut client, _catalog) = init().await;

        let mut workload_selectors = BTreeSet::new();
        workload_selectors.insert("PODLABELS:app:genericnode".to_string());

        let req = create_workload_jwts::Request {
            audiences: vec![SPIFFEID {
                trust_domain: "my trust domain".to_string(),
                path: "audiences".to_string(),
            }],
            selectors: workload_selectors,
            attestation_token: "dummy".to_string(),
            workload_spiffe_id: None,
        };

        let pod = get_pods();
        let node = get_nodes();
        let mut token_review = get_token_review();

        // Create failure to attest agent.
        token_review.status = None;

        client.queue_response(token_review).await;
        client.queue_response(pod).await;
        client.queue_response(node).await;

        let error = api.create_workload_jwts(req).await.unwrap_err();

        assert_matches!(error, Error::AttestAgent(_));
    }

    #[tokio::test]
    async fn create_new_jwts_match_identity_error() {
        let (api, _entries, _key_manager, _config, mut client, catalog) = init().await;

        let req = create_workload_jwts::Request {
            audiences: vec![SPIFFEID {
                trust_domain: "my trust domain".to_string(),
                path: "audiences".to_string(),
            }],
            selectors: BTreeSet::new(),
            attestation_token: "dummy".to_string(),
            workload_spiffe_id: None,
        };

        // Delete the parent, this will cause an error during matching since workload won't have any parent attached to it.
        catalog.batch_delete(&["parent".to_string()]).await.unwrap();

        let pod = get_pods();
        let node = get_nodes();
        let token_review = get_token_review();

        client.queue_response(token_review).await;
        client.queue_response(pod).await;
        client.queue_response(node).await;

        let error = api.create_workload_jwts(req).await.unwrap_err();

        assert_matches!(error, Error::MatchIdentity(_));
    }

    #[tokio::test]
    async fn create_new_jwts_jwt_factory_error() {
        let (api, _entries, key_manager, _config, mut client, _catalog) = init().await;

        let mut workload_selectors = BTreeSet::new();
        workload_selectors.insert("PODLABELS:app:genericnode".to_string());

        let req = create_workload_jwts::Request {
            audiences: vec![SPIFFEID {
                trust_domain: "my trust domain".to_string(),
                path: "audiences".to_string(),
            }],
            selectors: workload_selectors,
            attestation_token: "dummy".to_string(),
            workload_spiffe_id: None,
        };

        let pod = get_pods();
        let node = get_nodes();
        let token_review = get_token_review();

        client.queue_response(token_review).await;
        client.queue_response(pod).await;
        client.queue_response(node).await;

        // Create an error by deleting the key pair used for signing.
        let current_jwt_key = &key_manager.slots.read().await.current_jwt_key;
        let id = current_jwt_key.clone().id;
        key_manager.key_store.delete_key_pair(&id).await.unwrap();

        let error = api.create_workload_jwts(req).await.unwrap_err();

        assert_matches!(error, Error::CreateWorkloadJWT(_));
    }

    #[tokio::test]
    async fn get_trust_bundle_happy_path_test() {
        let (api, _entries, _key_manager, config, _client, _catalog) = init().await;

        let req = get_trust_bundle::Params {
            jwt_keys: true,
            x509_cas: true,
        };

        let response = api.get_trust_bundle(req).await.unwrap();
        let trust_bundle = response.trust_bundle;

        assert_eq!(config.trust_domain, trust_bundle.trust_domain);
        assert_eq!(1, trust_bundle.jwt_key_set.keys.len());
        assert_eq!(
            config.trust_bundle.refresh_hint,
            trust_bundle.jwt_key_set.spiffe_refresh_hint
        );
        assert_eq!(1, trust_bundle.jwt_key_set.spiffe_sequence_number);
    }
}
