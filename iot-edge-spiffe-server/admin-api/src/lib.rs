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

use catalog::Catalog;
use error::Error;
use http_common::Connector;
use server_admin_api::{
    create_registration_entries, delete_registration_entries, list_registration_entries, operation,
    select_get_registration_entries, update_registration_entries,
};
use server_config::Config;
use std::{io, path::Path, sync::Arc};

mod error;
mod http;

const SOCKET_DEFAULT_PERMISSION: u32 = 0o660;

pub async fn start_admin_api<C: Catalog + Send + Sync + 'static>(
    config: &Config,
    catalog: Arc<C>,
) -> Result<(), io::Error> {
    let api = Api { catalog };

    let service = http::Service { api: api.clone() };

    let connector = Connector::Unix {
        socket_path: Path::new(&config.socket_path).into(),
    };

    let mut incoming = connector.incoming(SOCKET_DEFAULT_PERMISSION, None).await?;

    // Channel to gracefully shut down the server. It's currently not used.
    let (_shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();

    let () = incoming.serve(service, shutdown_rx).await?;

    log::info!("Stopped server.");

    Ok(())
}

pub mod uri {
    pub const CREATE_DELETE_UPDATE_REGISTRATION_ENTRIES: &str = "/entries";
    pub const LIST_REGISTRATION_ENTRIES: &str = "/list-entries";
    pub const SELECT_GET_REGISTRATION_ENTRIES: &str = "/select-list-entries";
}

struct Api<C>
where
    C: Catalog + Send + Sync,
{
    catalog: Arc<C>,
}

impl<C> Clone for Api<C>
where
    C: Catalog + Send + Sync,
{
    fn clone(&self) -> Self {
        Self {
            catalog: self.catalog.clone(),
        }
    }
}

impl<C> Api<C>
where
    C: Catalog + Send + Sync,
{
    pub async fn create_registration_entries(
        &self,
        req: create_registration_entries::Request,
    ) -> create_registration_entries::Response {
        let mut results = Vec::new();

        for reg_entry in req.entries {
            let id = reg_entry.id.clone();

            let result = self
                .catalog
                .create_registration_entry(reg_entry)
                .await
                .map(|_| id.clone())
                .map_err(|err| operation::Error {
                    id,
                    error: format!("Error while creating entry: {}", err),
                });

            results.push(result);
        }

        create_registration_entries::Response { results }
    }

    pub async fn update_registration_entries(
        &self,
        req: update_registration_entries::Request,
    ) -> update_registration_entries::Response {
        let mut results = Vec::new();

        for reg_entry in req.entries {
            let id = reg_entry.id.clone();

            let result = self
                .catalog
                .update_registration_entry(reg_entry)
                .await
                .map(|_| id.clone())
                .map_err(|err| operation::Error::from(Error::UpdateEntry(Box::new(err), id)));

            results.push(result);
        }

        update_registration_entries::Response { results }
    }

    pub async fn select_list_registration_entries(
        &self,
        req: select_get_registration_entries::Request,
    ) -> select_get_registration_entries::Response {
        let mut results = Vec::new();

        for id in req.ids {
            let result = self
                .catalog
                .get_registration_entry(&id)
                .await
                .map_err(|err| operation::Error::from(Error::GetEntry(Box::new(err), id)));

            results.push(result);
        }

        select_get_registration_entries::Response { results }
    }

    pub async fn list_registration_entries(
        &self,
        params: list_registration_entries::Params,
    ) -> Result<list_registration_entries::Response, Error> {
        let page_size: usize = params
            .page_size
            .try_into()
            .map_err(|err| Error::InvalidPageSize(Box::new(err)))?;

        let (entries, next_page_token) = self
            .catalog
            .list_registration_entries(params.page_token, page_size)
            .await
            .map_err(|err| Error::ListEntry(Box::new(err)))?;

        let response = list_registration_entries::Response {
            entries,
            next_page_token,
        };

        Ok(response)
    }

    pub async fn delete_registration_entries(
        &self,
        req: delete_registration_entries::Request,
    ) -> delete_registration_entries::Response {
        let mut results = Vec::new();

        for id in req.ids {
            let result = self
                .catalog
                .delete_registration_entry(&id)
                .await
                .map(|_| id.clone())
                .map_err(|err| operation::Error::from(Error::DeleteEntry(Box::new(err), id)));

            results.push(result);
        }

        delete_registration_entries::Response { results }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use catalog::inmemory::InMemoryCatalog;
    use server_admin_api::RegistrationEntry;

    fn init() -> (Api<InMemoryCatalog>, Vec<RegistrationEntry>) {
        let catalog = Arc::new(catalog::inmemory::InMemoryCatalog::new());

        let api = Api { catalog };

        let entry = RegistrationEntry {
            id: String::from("id"),
            iot_hub_id: None,
            spiffe_id: String::from("spiffe id"),
            parent_id: None,
            selectors: [String::from("selector1"), String::from("selector2")].to_vec(),
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

        let res = api.create_registration_entries(req).await;

        for res in res.results {
            assert!(res.is_ok());
        }
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
        let res = api.create_registration_entries(req).await;

        for res in res.results {
            let res = res.unwrap_err();
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
        let res = api.update_registration_entries(req).await;

        for res in res.results {
            assert!(res.is_ok());
        }
    }

    #[tokio::test]
    pub async fn update_registration_entries_test_error_path() {
        let (api, entries) = init();

        let req = update_registration_entries::Request { entries };

        let res = api.update_registration_entries(req).await;
        for res in res.results {
            let res = res.unwrap_err();
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
        let res = api.delete_registration_entries(req).await;

        for res in res.results {
            assert!(res.is_ok());
        }
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
        let res = api.delete_registration_entries(req).await;

        for res in res.results {
            let res = res.unwrap_err();
            assert_eq!(res.id, "dummy".to_string());
        }
    }

    #[tokio::test]
    pub async fn list_registration_entries_test_happy_path() {
        let (api, mut entries) = init();
        let entry2 = RegistrationEntry {
            id: String::from("id2"),
            iot_hub_id: None,
            spiffe_id: String::from("spiffe id"),
            parent_id: None,
            selectors: [String::from("selector1"), String::from("selector2")].to_vec(),
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

        let req = list_registration_entries::Params {
            page_size: 1,
            page_token: None,
        };

        let res = api.list_registration_entries(req).await.unwrap();
        if res.entries[0].id != "id" {
            panic!("Invalid entry");
        }
        assert_eq!(res.entries.len(), 1);
        assert_eq!(res.next_page_token, Some("id2".to_string()));

        let req = list_registration_entries::Params {
            page_size: 1,
            page_token: Some("id2".to_string()),
        };
        let res = api.list_registration_entries(req).await.unwrap();
        if res.entries[0].id != "id2" {
            panic!("Invalid entry");
        }
        assert_eq!(res.entries.len(), 1);
        assert_eq!(res.next_page_token, None);

        let req = list_registration_entries::Params {
            page_size: 1,
            page_token: Some("j".to_string()),
        };
        let res = api.list_registration_entries(req).await.unwrap();
        assert_eq!(res.entries.len(), 0);
        assert_eq!(res.next_page_token, None);
    }

    #[tokio::test]
    pub async fn list_registration_entries_test_error_path() {
        let (api, mut entries) = init();
        let entry2 = RegistrationEntry {
            id: String::from("id2"),
            iot_hub_id: None,
            spiffe_id: String::from("spiffe id"),
            parent_id: None,
            selectors: [String::from("selector1"), String::from("selector2")].to_vec(),
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

        let req = list_registration_entries::Params {
            page_size: 0,
            page_token: None,
        };
        let _res = api.list_registration_entries(req).await.unwrap_err();
    }

    #[tokio::test]
    pub async fn select_list_registration_entries_test_happy_path() {
        let (api, mut entries) = init();
        let entry2 = RegistrationEntry {
            id: String::from("id2"),
            iot_hub_id: None,
            spiffe_id: String::from("spiffe id"),
            parent_id: None,
            selectors: [String::from("selector1"), String::from("selector2")].to_vec(),
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
