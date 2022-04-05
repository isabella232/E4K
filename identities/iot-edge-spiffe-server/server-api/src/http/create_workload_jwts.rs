// Copyright (c) Microsoft. All rights reserved.

use http::{Extensions, StatusCode};
use http_common::{server, DynRangeBounds};
use serde::de::IgnoredAny;
use server_agent_api::{create_workload_jwts, ApiVersion};
use std::borrow::Cow;

use crate::{error::Error, Api};

use super::uri;

pub(super) struct Route {
    api: Api,
}

#[async_trait::async_trait]
impl server::Route for Route {
    type ApiVersion = ApiVersion;
    type Service = super::Service;
    type DeleteBody = IgnoredAny;
    type PostBody = create_workload_jwts::Request;
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
        if path != uri::CREATE_WORKLOAD_JTWS {
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

        let res = self.api.create_workload_jwts(body).await;
        let res = match res {
            Ok(res) => res,
            Err(err) => {
                if let Error::AttestAgent(_) = err {
                    return Err(server::Error {
                        status_code: StatusCode::FORBIDDEN,
                        message: format!("Error doing agent attestation: {}", err).into(),
                    });
                }

                return Err(server::Error {
                    status_code: StatusCode::INTERNAL_SERVER_ERROR,
                    message: format!("Error when creating new jwt: {}", err).into(),
                });
            }
        };

        let res = server::response::json(StatusCode::CREATED, &res);

        Ok(res)
    }
}
