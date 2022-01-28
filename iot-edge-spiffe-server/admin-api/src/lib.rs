// Copyright (c) Microsoft. All rights reserved.

#![deny(rust_2018_idioms)]
#![warn(clippy::all, clippy::pedantic)]
#![allow(
    clippy::default_trait_access,
    clippy::let_and_return,
    clippy::let_unit_value,
    clippy::missing_errors_doc,
    clippy::similar_names,
    clippy::too_many_lines
)]

use catalog::Catalog;
use common_admin_api::{
    create_registration_entries, delete_registration_entries, list_registration_entries, operation,
    select_list_registration_entries, update_registration_entries,
};
use error::Error;
use futures_util::lock::Mutex;
use http_common::Connector;
use server_config::Config;
use std::{io, sync::Arc};

mod error;
mod http;

const SOCKET_DEFAULT_PERMISSION: u32 = 0o660;

pub async fn start_admin_api(
    config: &Config,
    catalog: Box<dyn Catalog + Send + Sync>,
) -> Result<(), io::Error> {
    let api = Api { catalog };

    let api = Arc::new(Mutex::new(api));
    let service = http::Service { api };

    let connector = Connector::Unix {
        socket_path: std::path::Path::new(&config.socket_path).into(),
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
    pub const LIST_REGISTRATION_ENTRIES: &str = "/listEntries";
    pub const SELECT_LIST_REGISTRATION_ENTRIES: &str = "/selectListEntries";
}

struct Api {
    catalog: Box<dyn Catalog + Send + Sync>,
}

impl Api {
    pub async fn create_registration_entries(
        &mut self,
        req: create_registration_entries::Request,
    ) -> create_registration_entries::Response {
        let mut results = Vec::new();

        for reg_entry in req.entries {
            let id = reg_entry.id.clone();

            let result = self.catalog.create_registration_entry(reg_entry).await;
            let result = match result {
                Ok(_) => Ok(id),
                Err(err) => Err(operation::Error {
                    id,
                    error: operation::Status::DuplicatedEntry(format!(
                        "Error while creating entry: {}",
                        err
                    )),
                }),
            };

            results.push(result);
        }

        let response = create_registration_entries::Response { results };

        response
    }

    pub async fn update_registration_entries(
        &mut self,
        req: update_registration_entries::Request,
    ) -> update_registration_entries::Response {
        let mut results = Vec::new();

        for reg_entry in req.entries {
            let id = reg_entry.id.clone();

            let result = self.catalog.update_registration_entry(reg_entry).await;
            let result = match result {
                Ok(_) => Ok(id),
                Err(err) => Err(operation::Error {
                    id,
                    error: operation::Status::EntryDoNotExist(format!(
                        "Error while creating entry: {}",
                        err
                    )),
                }),
            };

            results.push(result);
        }

        let response = update_registration_entries::Response { results };

        response
    }

    pub async fn select_list_registration_entries(
        &self,
        req: select_list_registration_entries::Request,
    ) -> select_list_registration_entries::Response {
        let mut results = Vec::new();

        for id in req.ids {
            let result = self.catalog.get_registration_entry(&id).await;
            let result = result.map_err(|err| operation::Error {
                id,
                error: operation::Status::EntryDoNotExist(format!(
                    "Error while getting entry: {}",
                    err
                )),
            });

            results.push(result);
        }

        let response = select_list_registration_entries::Response { results };

        response
    }

    pub async fn list_registration_entries(
        &self,
        req: list_registration_entries::Request,
    ) -> Result<list_registration_entries::Response, Error> {
        let page_size: usize = req
            .page_size
            .try_into()
            .map_err(|_| Error::InvalidArguments("Page size is too big".to_string()))?;
        let page_number: usize = req
            .page_number
            .try_into()
            .map_err(|_| Error::InvalidArguments("Page size is too big".to_string()))?;

        let (entries, next_page_number) = match self
            .catalog
            .list_registration_entries(page_number, page_size)
            .await
        {
            Ok(resp) => resp,
            Err(err) => {
                return Err(Error::CatalogError(format!(
                    "Error while listing registration entries: {}",
                    err
                )))
            }
        };

        let next_page_number =
            next_page_number.map(|x| u32::try_from(x).expect("Cannot convert back to u32"));
        let response = list_registration_entries::Response {
            entries,
            next_page_number,
        };

        Ok(response)
    }

    pub async fn delete_registration_entries(
        &mut self,
        req: delete_registration_entries::Request,
    ) -> delete_registration_entries::Response {
        let mut results = Vec::new();

        for id in req.ids {
            let result = self.catalog.delete_registration_entry(&id).await;
            let result = match result {
                Ok(_) => Ok(id),
                Err(err) => Err(operation::Error {
                    id,
                    error: operation::Status::EntryDoNotExist(format!(
                        "Error while removing entry: {}",
                        err
                    )),
                }),
            };
            results.push(result);
        }

        let response = delete_registration_entries::Response { results };

        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common_admin_api::RegistrationEntry;

    fn init() -> (Api, Vec<RegistrationEntry>) {
        let catalog = catalog::load_catalog();

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
        let (mut api, entries) = init();

        let req = create_registration_entries::Request { entries };

        let res = api.create_registration_entries(req).await;

        for res in res.results {
            assert!(res.is_ok());
        }
    }

    #[tokio::test]
    pub async fn create_registration_entries_test_error_path() {
        let (mut api, entries) = init();

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
            if let operation::Status::DuplicatedEntry(_) = res.error {
            } else {
                panic!("Wrong error type returned for create_registration_entry")
            };
        }
    }

    #[tokio::test]
    pub async fn update_registration_entries_test_happy_path() {
        let (mut api, entries) = init();

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
        let (mut api, entries) = init();

        let req = update_registration_entries::Request { entries };

        let res = api.update_registration_entries(req).await;
        for res in res.results {
            let res = res.unwrap_err();
            assert_eq!(res.id, "id".to_string());
            if let operation::Status::EntryDoNotExist(_) = res.error {
            } else {
                panic!("Wrong error type returned for create_registration_entry")
            };
        }
    }

    #[tokio::test]
    pub async fn delete_registration_entries_test_happy_path() {
        let (mut api, entries) = init();

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
        let (mut api, entries) = init();

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
            if let operation::Status::EntryDoNotExist(_) = res.error {
            } else {
                panic!("Wrong error type returned for create_registration_entry")
            };
        }
    }

    #[tokio::test]
    pub async fn list_registration_entries_test_happy_path() {
        let (mut api, mut entries) = init();
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

        let req = list_registration_entries::Request {
            page_size: 1,
            page_number: 0,
        };

        let res = api.list_registration_entries(req).await.unwrap();
        if (res.entries[0].id != "id") && (res.entries[0].id != "id2") {
            panic!("Invalid entry");
        }
        assert_eq!(res.entries.len(), 1);
        assert_eq!(res.next_page_number, Some(1));

        let req = list_registration_entries::Request {
            page_size: 1,
            page_number: 1,
        };
        let res = api.list_registration_entries(req).await.unwrap();
        if (res.entries[0].id != "id") && (res.entries[0].id != "id2") {
            panic!("Invalid entry");
        }
        assert_eq!(res.entries.len(), 1);
        assert_eq!(res.next_page_number, None);
    }

    #[tokio::test]
    pub async fn list_registration_entries_test_error_path() {
        let (mut api, mut entries) = init();
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

        let req = list_registration_entries::Request {
            page_size: 0,
            page_number: 0,
        };
        let res = api.list_registration_entries(req).await.unwrap_err();

        if let Error::CatalogError(_) = res {
        } else {
            panic!("Wrong error type returned for list_registration_entries")
        };

        let req = list_registration_entries::Request {
            page_size: 1,
            page_number: 2,
        };
        let res = api.list_registration_entries(req).await.unwrap_err();
        if let Error::CatalogError(_) = res {
        } else {
            panic!("Wrong error type returned for list_registration_entries")
        };
    }
}
