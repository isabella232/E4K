// Copyright (c) Microsoft. All rights reserved.

use catalog::{Entries, TrustBundleStore};
use http::{Extensions, StatusCode};
use http_common::{server, DynRangeBounds};
use key_store::KeyStore;
use serde::de::IgnoredAny;
use server_agent_api::{create_new_jwt, ApiVersion};
use std::borrow::Cow;

use crate::{uri, Api};

pub(super) struct Route<C, D>
where
    C: Entries + TrustBundleStore + Send + Sync + 'static,
    D: KeyStore + Send + Sync + 'static,
{
    api: Api<C, D>,
}

#[async_trait::async_trait]
impl<C, D> server::Route for Route<C, D>
where
    C: Entries + TrustBundleStore + Send + Sync + 'static,
    D: KeyStore + Send + Sync + 'static,
{
    type ApiVersion = ApiVersion;
    type Service = super::Service<C, D>;
    type DeleteBody = IgnoredAny;
    type PostBody = create_new_jwt::Request;
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
        if path != uri::CREATE_NEW_JTW {
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

        let res = self.api.create_new_jwt(body).await;

        let res = server::response::json(StatusCode::CREATED, &res);

        Ok(res)
    }
}
