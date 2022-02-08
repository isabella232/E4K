// Copyright (c) Microsoft. All rights reserved.

use std::borrow::Cow;

use catalog::{Entries, TrustBundleStore};
use http::{Extensions, StatusCode};
use http_common::{server, DynRangeBounds};
use key_store::KeyStore;
use serde::de::IgnoredAny;
use server_agent_api::{get_trust_bundle, ApiVersion};

use crate::{uri, Api};

pub(super) struct Route<C, D>
where
    C: Entries + TrustBundleStore + Send + Sync + 'static,
    D: KeyStore + Send + Sync + 'static,
{
    x509_cas: Option<String>,
    jwt_keys: Option<String>,
    api: Api<C, D>,
}

#[async_trait::async_trait]
impl<C, D> server::Route for Route<C, D>
where
    C: Entries + TrustBundleStore + Send + Sync + 'static,
    D: KeyStore + Send + Sync + 'static,
{
    type ApiVersion = ApiVersion;
    type DeleteBody = IgnoredAny;
    type PostBody = IgnoredAny;
    type Service = super::Service<C, D>;
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
        if path != uri::GET_TRUST_BUNDLE {
            return None;
        }

        let mut x509_cas: Option<String> = None;
        let mut jwt_keys: Option<String> = None;

        for q in query.iter() {
            x509_cas = if q.0 == "x509_cas" {
                Some(q.1.to_string())
            } else {
                None
            };

            jwt_keys = if q.0 == "jwt_keys" {
                Some(q.1.to_string())
            } else {
                None
            };
        }

        Some(Route {
            x509_cas,
            jwt_keys,
            api: service.api.clone(),
        })
    }

    async fn get(self) -> server::RouteResponse {
        let jwt_keys = if let Some(jwt_keys) = self.jwt_keys {
            jwt_keys.parse::<bool>().map_err(|_| server::Error {
                status_code: StatusCode::BAD_REQUEST,
                message: "Could not convert jwt_keys to bool".into(),
            })?
        } else {
            false
        };

        let x509_cas = if let Some(x509_cas) = self.x509_cas {
            x509_cas.parse::<bool>().map_err(|_| server::Error {
                status_code: StatusCode::BAD_REQUEST,
                message: "Could not convert x509_cas to bool".into(),
            })?
        } else {
            false
        };

        let params = get_trust_bundle::Params { jwt_keys, x509_cas };

        let res = self.api.get_trust_bundle(params).await;

        let res = server::response::json(StatusCode::CREATED, &res);

        Ok(res)
    }
}
