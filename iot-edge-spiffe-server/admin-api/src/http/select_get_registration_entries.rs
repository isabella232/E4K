// Copyright (c) Microsoft. All rights reserved.
use crate::{uri, Api};
use common_admin_api::{select_get_registration_entries, ApiVersion};
use http::{Extensions, StatusCode};
use http_common::{server, DynRangeBounds};
use serde::de::IgnoredAny;
use std::borrow::Cow;

pub(super) struct Route {
    api: Api,
}

#[async_trait::async_trait]
impl server::Route for Route {
    type ApiVersion = ApiVersion;
    type Service = super::Service;
    type DeleteBody = IgnoredAny;
    type PostBody = select_get_registration_entries::Request;
    type PutBody = IgnoredAny;

    fn api_version() -> &'static dyn DynRangeBounds<Self::ApiVersion> {
        &((ApiVersion::V2022_06_01)..)
    }

    fn from_uri(
        service: &Self::Service,
        path: &str,
        _query: &[(Cow<'_, str>, Cow<'_, str>)],
        _extensions: &Extensions,
    ) -> Option<Self> {
        if path != uri::SELECT_GET_REGISTRATION_ENTRIES {
            return None;
        }
        Some(Route {
            api: service.api.clone(),
        })
    }

    async fn post(self, body: Option<Self::PostBody>) -> server::RouteResponse {
        let body = body.ok_or_else(|| server::Error {
            status_code: StatusCode::BAD_REQUEST,
            message: "missing request body".into(),
        })?;

        let res = self.api.select_list_registration_entries(body).await;

        let res = server::response::json(StatusCode::OK, &res);

        Ok(res)
    }
}
