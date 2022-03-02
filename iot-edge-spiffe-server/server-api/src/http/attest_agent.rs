// Copyright (c) Microsoft. All rights reserved.

use http::{Extensions, StatusCode};
use http_common::{server, DynRangeBounds};
use serde::de::IgnoredAny;
use server_agent_api::ApiVersion;
use std::borrow::Cow;

use crate::Api;

use super::uri;

pub(super) struct Route {
    token: Option<String>,
    api: Api,
}

#[async_trait::async_trait]
impl server::Route for Route {
    type ApiVersion = ApiVersion;
    type Service = super::Service;
    type DeleteBody = IgnoredAny;
    type PostBody = IgnoredAny;
    type PutBody = IgnoredAny;

    fn api_version() -> &'static dyn DynRangeBounds<Self::ApiVersion> {
        &((ApiVersion::V2022_06_01)..)
    }

    fn from_uri(
        service: &Self::Service,
        path: &str,
        query: &[(Cow<'_, str>, Cow<'_, str>)],
        _extensions: &Extensions,
    ) -> Option<Self> {
        if path != uri::ATTEST_AGENT {
            return None;
        }

        let mut token: Option<String> = None;

        for q in query.iter() {
            if &q.0 as &str == "token" {
                token = Some(q.1.to_string());
            }
        }

        Some(Route {
            token,
            api: service.api.clone(),
        })
    }

    async fn get(self) -> server::RouteResponse {
        let token = if let Some(token) = self.token {
            token
        } else {
            return Err(server::Error {
                status_code: StatusCode::UNAUTHORIZED,
                message: "missing auth token".into(),
            });
        };

        let res = self.api.attest_agent(&token).await;
        let res = match res {
            Ok(res) => res,
            Err(err) => {
                return Err(server::Error {
                    status_code: StatusCode::UNAUTHORIZED,
                    message: format!("Error while attesting agent: {}", err).into(),
                })
            }
        };

        let res = server::response::json(StatusCode::CREATED, &res);

        Ok(res)
    }
}
