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
    select_list_registration_entries,
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
    pub const CREATE_DELETE_REGISTRATION_ENTRIES: &str = "/entries";
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
                    error: operation::Status::DuplicateEntry(format!(
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
        let (entries, next_page_number) = match self
            .catalog
            .list_registration_entries(req.page_number, req.page_size)
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
