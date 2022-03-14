// Copyright (c) Microsoft. All rights reserved.

use crate::{error::Error, Api};
use server_admin_api::{
    create_registration_entries, delete_registration_entries, list_all, operation,
    select_get_registration_entries, update_registration_entries,
};

impl Api {
    pub async fn create_registration_entries(
        &self,
        req: create_registration_entries::Request,
    ) -> create_registration_entries::Response {
        let results = self
            .catalog
            .batch_create(req.entries)
            .await
            .map_err(|err| err.into_iter().map(operation::Error::from).collect());

        create_registration_entries::Response { results }
    }

    pub async fn update_registration_entries(
        &self,
        req: update_registration_entries::Request,
    ) -> update_registration_entries::Response {
        let results = self
            .catalog
            .batch_update(req.entries)
            .await
            .map_err(|err| err.into_iter().map(operation::Error::from).collect());

        update_registration_entries::Response { results }
    }

    pub async fn select_list_registration_entries(
        &self,
        req: select_get_registration_entries::Request,
    ) -> select_get_registration_entries::Response {
        let mut results = Vec::new();

        let catalog_results = self.catalog.batch_get(&req.ids).await;

        for (id, result) in catalog_results {
            let result = result.map_err(|err| operation::Error::from((id, err)));

            results.push(result);
        }

        select_get_registration_entries::Response { results }
    }

    pub async fn list_all(&self, params: list_all::Params) -> Result<list_all::Response, Error> {
        let page_size: usize = params
            .page_size
            .try_into()
            .map_err(|err| Error::InvalidPageSize(Box::new(err)))?;

        let (entries, next_page_token) = self
            .catalog
            .list_all(params.page_token, page_size)
            .await
            .map_err(|err| Error::ListEntry(err))?;

        let response = list_all::Response {
            entries,
            next_page_token,
        };

        Ok(response)
    }

    pub async fn delete_registration_entries(
        &self,
        req: delete_registration_entries::Request,
    ) -> delete_registration_entries::Response {
        let results = self
            .catalog
            .batch_delete(&req.ids)
            .await
            .map_err(|err| err.into_iter().map(operation::Error::from).collect());

        delete_registration_entries::Response { results }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use core_objects::{
        build_selector_string, AttestationConfig, EntryNodeAttestation, NodeAttestationPlugin,
        NodeSelectorType, RegistrationEntry, SPIFFEID,
    };

    use crate::Api;

    use super::*;

    fn init() -> (Api, Vec<RegistrationEntry>) {
        let catalog = Arc::new(catalog::inmemory::Catalog::new());

        let api = Api { catalog };
        let spiffe_id = SPIFFEID {
            trust_domain: "trust_domain".to_string(),
            path: "path".to_string(),
        };

        let entry = RegistrationEntry {
            id: String::from("id"),
            other_identities: Vec::new(),
            spiffe_id,
            attestation_config: AttestationConfig::Node(EntryNodeAttestation {
                value: vec![
                    build_selector_string(&NodeSelectorType::Cluster, "selector1"),
                    build_selector_string(&NodeSelectorType::AgentNameSpace, "selector2"),
                ],
                plugin: NodeAttestationPlugin::Sat,
            }),
            admin: false,
            ttl: 0,
            expires_at: 0,
            dns_names: Vec::new(),
            revision_number: 0,
            store_svid: false,
        };
        let entries = vec![entry];

        (api, entries)
    }

    #[tokio::test]
    pub async fn create_registration_entries_test_happy_path() {
        let (api, entries) = init();

        let req = create_registration_entries::Request { entries };

        api.create_registration_entries(req).await.results.unwrap();
    }

    #[tokio::test]
    pub async fn create_registration_entries_test_error_path() {
        let (api, entries) = init();

        let req = create_registration_entries::Request {
            entries: entries.clone(),
        };
        let _res = api.create_registration_entries(req).await;

        let req = create_registration_entries::Request {
            entries: entries.clone(),
        };
        let res = api
            .create_registration_entries(req)
            .await
            .results
            .unwrap_err();

        for res in res {
            assert_eq!(res.id, "id".to_string());
        }
    }

    #[tokio::test]
    pub async fn update_registration_entries_test_happy_path() {
        let (api, entries) = init();

        let req = create_registration_entries::Request {
            entries: entries.clone(),
        };
        let _res = api.create_registration_entries(req).await;

        let req = update_registration_entries::Request {
            entries: entries.clone(),
        };
        api.update_registration_entries(req).await.results.unwrap();
    }

    #[tokio::test]
    pub async fn update_registration_entries_test_error_path() {
        let (api, entries) = init();

        let req = update_registration_entries::Request { entries };

        let res = api
            .update_registration_entries(req)
            .await
            .results
            .unwrap_err();
        for res in res {
            assert_eq!(res.id, "id".to_string());
        }
    }

    #[tokio::test]
    pub async fn delete_registration_entries_test_happy_path() {
        let (api, entries) = init();

        let mut ids = Vec::new();
        for entry in &entries {
            ids.push(entry.id.clone());
        }
        let req = create_registration_entries::Request { entries };

        let _res = api.create_registration_entries(req).await;
        let req = delete_registration_entries::Request { ids };
        api.delete_registration_entries(req).await.results.unwrap();
    }

    #[tokio::test]
    pub async fn delete_registration_entries_test_error_path() {
        let (api, entries) = init();

        let mut ids = Vec::new();
        for _entry in &entries {
            ids.push("dummy".to_string());
        }
        let req = create_registration_entries::Request { entries };

        let _res = api.create_registration_entries(req).await;
        let req = delete_registration_entries::Request { ids };
        let res = api
            .delete_registration_entries(req)
            .await
            .results
            .unwrap_err();

        for res in res {
            assert_eq!(res.id, "dummy".to_string());
        }
    }

    #[tokio::test]
    pub async fn list_registration_entries_test_happy_path() {
        let (api, mut entries) = init();
        let spiffe_id = SPIFFEID {
            trust_domain: "trust_domain".to_string(),
            path: "path".to_string(),
        };
        let entry2 = RegistrationEntry {
            id: String::from("id2"),
            other_identities: Vec::new(),
            spiffe_id,
            attestation_config: AttestationConfig::Node(EntryNodeAttestation {
                value: vec![
                    build_selector_string(&NodeSelectorType::Cluster, "selector1"),
                    build_selector_string(&NodeSelectorType::AgentNameSpace, "selector2"),
                ],
                plugin: NodeAttestationPlugin::Sat,
            }),
            admin: false,
            ttl: 0,
            expires_at: 0,
            dns_names: Vec::new(),
            revision_number: 0,
            store_svid: false,
        };
        entries.push(entry2);

        let req = create_registration_entries::Request {
            entries: entries.clone(),
        };
        let _res = api.create_registration_entries(req).await;

        let req = list_all::Params {
            page_size: 1,
            page_token: None,
        };

        let res = api.list_all(req).await.unwrap();
        if res.entries[0].id != "id" {
            panic!("Invalid entry");
        }
        assert_eq!(res.entries.len(), 1);
        assert_eq!(res.next_page_token, Some("id2".to_string()));

        let req = list_all::Params {
            page_size: 1,
            page_token: Some("id2".to_string()),
        };
        let res = api.list_all(req).await.unwrap();
        if res.entries[0].id != "id2" {
            panic!("Invalid entry");
        }
        assert_eq!(res.entries.len(), 1);
        assert_eq!(res.next_page_token, None);

        let req = list_all::Params {
            page_size: 1,
            page_token: Some("j".to_string()),
        };
        let res = api.list_all(req).await.unwrap();
        assert_eq!(res.entries.len(), 0);
        assert_eq!(res.next_page_token, None);
    }

    #[tokio::test]
    pub async fn list_registration_entries_test_error_path() {
        let (api, mut entries) = init();
        let spiffe_id = SPIFFEID {
            trust_domain: "trust_domain".to_string(),
            path: "path".to_string(),
        };
        let entry2 = RegistrationEntry {
            id: String::from("id2"),
            other_identities: Vec::new(),
            spiffe_id,
            attestation_config: AttestationConfig::Node(EntryNodeAttestation {
                value: vec![
                    build_selector_string(&NodeSelectorType::Cluster, "selector1"),
                    build_selector_string(&NodeSelectorType::AgentNameSpace, "selector2"),
                ],
                plugin: NodeAttestationPlugin::Sat,
            }),
            admin: false,
            ttl: 0,
            expires_at: 0,
            dns_names: Vec::new(),
            revision_number: 0,
            store_svid: false,
        };
        entries.push(entry2);

        let req = create_registration_entries::Request {
            entries: entries.clone(),
        };
        let _res = api.create_registration_entries(req).await;

        let req = list_all::Params {
            page_size: 0,
            page_token: None,
        };
        let _res = api.list_all(req).await.unwrap_err();
    }

    #[tokio::test]
    pub async fn select_list_registration_entries_test_happy_path() {
        let (api, mut entries) = init();
        let spiffe_id = SPIFFEID {
            trust_domain: "trust_domain".to_string(),
            path: "path".to_string(),
        };
        let entry2 = RegistrationEntry {
            id: String::from("id2"),
            other_identities: Vec::new(),
            spiffe_id,
            attestation_config: AttestationConfig::Node(EntryNodeAttestation {
                value: vec![
                    build_selector_string(&NodeSelectorType::Cluster, "selector1"),
                    build_selector_string(&NodeSelectorType::AgentNameSpace, "selector2"),
                ],
                plugin: NodeAttestationPlugin::Sat,
            }),
            admin: false,
            ttl: 0,
            expires_at: 0,
            dns_names: Vec::new(),
            revision_number: 0,
            store_svid: false,
        };
        entries.push(entry2);

        let req = create_registration_entries::Request { entries };

        let _res = api.create_registration_entries(req).await;

        let ids = vec!["id".to_string(), "id2".to_string()];
        let req = select_get_registration_entries::Request { ids };
        let res = api.select_list_registration_entries(req).await;
        let results = res.results;

        assert_eq!(2, results.len());
        for res in results {
            assert!(res.is_ok());
        }

        let ids = vec!["id".to_string()];
        let req = select_get_registration_entries::Request { ids };
        let res = api.select_list_registration_entries(req).await;
        let results = res.results;
        assert_eq!(1, results.len());
    }
}
