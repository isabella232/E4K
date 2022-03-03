// Copyright (c) Microsoft. All rights reserved.

use server_agent_api::{create_workload_jwt, get_trust_bundle};
use svid_factory::JWTSVIDParams;

use crate::{error::Error, Api};

impl Api {
    pub async fn create_workload_jwt(
        &self,
        req: create_workload_jwt::Request,
    ) -> Result<create_workload_jwt::Response, Error> {
        //TODO !! caller spiffeid
        //TODO !! validate request
        //TODO !! Validate caller has right to get the jwt

        let results = self.catalog.batch_get(&[req.id.clone()]).await;

        let (_id, entry) = results.into_iter().next().ok_or(Error::InvalidResponse)?;

        let entry = entry.map_err(Error::CatalogGetEntry)?;

        let jwt_svid_params = JWTSVIDParams {
            spiffe_id: entry.spiffe_id,
            audiences: req.audiences,
            other_identities: entry.other_identities,
        };

        let jwt_svid = self
            .svid_factory
            .create_jwt_svid(jwt_svid_params)
            .await
            .map_err(Error::CreateWorkloadJWT)?;

        Ok(create_workload_jwt::Response { jwt_svid })
    }

    pub async fn get_trust_bundle(
        &self,
        params: get_trust_bundle::Request,
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
    use catalog::inmemory;
    use core_objects::{
        AttestationConfig, EntryNodeAttestation, NodeAttestationPlugin, RegistrationEntry,
        CONFIG_DEFAULT_PATH, SPIFFEID,
    };
    use key_manager::KeyManager;
    use key_store::disk;
    use matches::assert_matches;
    use mock_kube::Client;
    use node_attestation_server::NodeAttestatorFactory;
    use server_config::{Config, KeyStoreConfig, KeyStoreConfigDisk};
    use svid_factory::SVIDFactory;
    use trust_bundle_builder::TrustBundleBuilder;

    use std::sync::Arc;
    use tempdir::TempDir;

    async fn init() -> (Api, Vec<RegistrationEntry>, Arc<KeyManager>, Config) {
        let mut config = Config::load_config(CONFIG_DEFAULT_PATH).unwrap();
        let dir = TempDir::new("test").unwrap();
        let key_base_path = dir.into_path().to_str().unwrap().to_string();
        let key_plugin = KeyStoreConfigDisk {
            key_base_path: key_base_path.clone(),
        };

        let spiffe_id = SPIFFEID {
            trust_domain: "trust_domain".to_string(),
            path: "path".to_string(),
        };

        let entry = RegistrationEntry {
            id: String::from("id"),
            other_identities: Vec::new(),
            spiffe_id,
            attestation_config: AttestationConfig::Node(EntryNodeAttestation {
                value: Vec::new(),
                plugin: NodeAttestationPlugin::Psat,
            }),
            admin: false,
            ttl: 0,
            expires_at: 0,
            dns_names: Vec::new(),
            revision_number: 0,
            store_svid: false,
        };
        let entries = vec![entry];

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
        let client = Client::try_default().await.unwrap();
        let node_attestation = NodeAttestatorFactory::get(
            &config.node_attestation_config,
            &config.trust_domain,
            client,
        );

        let api = Api {
            catalog,
            svid_factory,
            trust_bundle_builder,
            node_attestation,
            iotedge_server_spiffe_id,
        };

        (api, entries, key_manager, config)
    }

    #[tokio::test]
    async fn create_new_jwt_happy_path() {
        let (api, entries, _key_manager, _config) = init().await;

        api.catalog.batch_create(entries.clone()).await.unwrap();

        let entry = entries[0].clone();

        let req = create_workload_jwt::Request {
            id: entry.id.clone(),
            audiences: [SPIFFEID {
                trust_domain: "my trust domain".to_string(),
                path: "audiences".to_string(),
            }]
            .to_vec(),
        };

        let response = api.create_workload_jwt(req).await.unwrap();

        assert_eq!(
            response.jwt_svid.spiffe_id.trust_domain,
            entry.spiffe_id.trust_domain
        );
        assert_eq!(response.jwt_svid.spiffe_id.path, entry.spiffe_id.path);
    }

    #[tokio::test]
    async fn create_new_jwt_get_entry_error() {
        let (api, entries, _key_manager, _config) = init().await;

        let entry = entries[0].clone();

        let req = create_workload_jwt::Request {
            id: entry.id.clone(),
            audiences: [SPIFFEID {
                trust_domain: "my trust domain".to_string(),
                path: "audiences".to_string(),
            }]
            .to_vec(),
        };

        let error = api.create_workload_jwt(req).await.unwrap_err();

        assert_matches!(error, Error::CatalogGetEntry(_));
    }

    #[tokio::test]
    async fn create_new_jwt_jwt_factory_error() {
        let (api, entries, key_manager, _config) = init().await;

        api.catalog.batch_create(entries.clone()).await.unwrap();

        let entry = entries[0].clone();

        let req = create_workload_jwt::Request {
            id: entry.id.clone(),
            audiences: [SPIFFEID {
                trust_domain: "my trust domain".to_string(),
                path: "audiences".to_string(),
            }]
            .to_vec(),
        };

        let current_jwt_key = &key_manager.slots.read().await.current_jwt_key;
        let id = current_jwt_key.clone().id;
        key_manager.key_store.delete_key_pair(&id).await.unwrap();

        let error = api.create_workload_jwt(req).await.unwrap_err();

        assert_matches!(error, Error::CreateWorkloadJWT(_));
    }

    #[tokio::test]
    async fn get_trust_bundle_happy_path_test() {
        let (api, _entries, _key_manager, config) = init().await;

        let req = get_trust_bundle::Request {
            jwt_keys: true,
            x509_cas: true,
        };

        let response = api.get_trust_bundle(req).await.unwrap();
        let trust_bundle = response.trust_bundle;

        assert_eq!(config.trust_domain, trust_bundle.trust_domain);
        assert_eq!(1, trust_bundle.jwt_keys.len());
        assert_eq!(config.trust_bundle.refresh_hint, trust_bundle.refresh_hint);
        assert_eq!(1, trust_bundle.sequence_number);
    }
}
