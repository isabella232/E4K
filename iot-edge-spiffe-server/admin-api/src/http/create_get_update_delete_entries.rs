// Copyright (c) Microsoft. All rights reserved.

// This file does all the edit operation on entries: Create, Update and Delete.
// Because Get also requires a post, it is in another file.

use std::borrow::Cow;

use crate::{uri, Api};
use catalog::Entries;
use http::{Extensions, StatusCode};
use http_common::{server, DynRangeBounds};
use server_admin_api::{
    create_registration_entries, delete_registration_entries, list_all,
    update_registration_entries, ApiVersion,
};

pub(super) struct Route<C>
where
    C: Entries + Send + Sync + 'static,
{
    page_size: Option<String>,
    page_token: Option<String>,
    api: Api<C>,
}

#[async_trait::async_trait]
impl<C> server::Route for Route<C>
where
    C: Entries + Send + Sync + 'static,
{
    type ApiVersion = ApiVersion;
    type Service = super::Service<C>;
    type DeleteBody = delete_registration_entries::Request;
    type PostBody = create_registration_entries::Request;
    type PutBody = update_registration_entries::Request;

    fn api_version() -> &'static dyn DynRangeBounds<Self::ApiVersion> {
        &((ApiVersion::V2022_06_01)..)
    }

    fn from_uri(
        service: &Self::Service,
        path: &str,
        query: &[(Cow<'_, str>, Cow<'_, str>)],
        _extensions: &Extensions,
    ) -> Option<Self> {
        if path != uri::CREATE_DELETE_UPDATE_REGISTRATION_ENTRIES {
            return None;
        }

        let mut page_size: Option<String> = None;
        let mut page_token: Option<String> = None;

        for q in query.iter() {
            match &q.0 as &str {
                "page_size" => page_size = Some(q.1.to_string()),
                "page_token" => page_token = Some(q.1.to_string()),
                _ => {}
            }
        }

        Some(Route {
            page_size,
            page_token,
            api: service.api.clone(),
        })
    }

    async fn get(self) -> server::RouteResponse {
        let page_size = self
            .page_size
            .ok_or(server::Error {
                status_code: StatusCode::BAD_REQUEST,
                message: "Please provide the page size parameter".into(),
            })?
            .parse::<u32>()
            .map_err(|_| server::Error {
                status_code: StatusCode::BAD_REQUEST,
                message: "Could not convert page size to u32".into(),
            })?;

        let params = list_all::Params {
            page_size,
            page_token: self.page_token,
        };

        let res = self.api.list_all(params).await;
        let res = match res {
            Ok(res) => res,
            Err(err) => {
                return Err(server::Error {
                    status_code: StatusCode::BAD_REQUEST,
                    message: format!("Error proccessing listing entries request: {}", err).into(),
                })
            }
        };

        let res = server::response::json(StatusCode::OK, &res);

        Ok(res)
    }

    async fn delete(self, body: Option<Self::DeleteBody>) -> server::RouteResponse {
        let body = body.ok_or_else(|| server::Error {
            status_code: StatusCode::BAD_REQUEST,
            message: "missing request body".into(),
        })?;

        let res = self.api.delete_registration_entries(body).await;

        let res = server::response::json(StatusCode::OK, &res);

        Ok(res)
    }

    async fn post(self, body: Option<Self::PostBody>) -> server::RouteResponse {
        let body = body.ok_or_else(|| server::Error {
            status_code: StatusCode::BAD_REQUEST,
            message: "missing request body".into(),
        })?;

        let res = self.api.create_registration_entries(body).await;

        let res = server::response::json(StatusCode::CREATED, &res);

        Ok(res)
    }

    async fn put(self, body: Self::PutBody) -> server::RouteResponse {
        let res = self.api.update_registration_entries(body).await;

        let res = server::response::json(StatusCode::OK, &res);

        Ok(res)
    }
}
