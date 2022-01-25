// Copyright (c) Microsoft. All rights reserved.

pub(super) struct Route {
    api: std::sync::Arc<futures_util::lock::Mutex<crate::Api>>,
}

#[async_trait::async_trait]
impl http_common::server::Route for Route {
    type ApiVersion = common_admin_api::ApiVersion;
    fn api_version() -> &'static dyn http_common::DynRangeBounds<Self::ApiVersion> {
        &((common_admin_api::ApiVersion::V2020_09_01)..)
    }

    type Service = super::Service;
    fn from_uri(
        service: &Self::Service,
        path: &str,
        _query: &[(std::borrow::Cow<'_, str>, std::borrow::Cow<'_, str>)],
        _extensions: &http::Extensions,
    ) -> Option<Self> {
        if path != crate::uri::CREATE_DELETE_REGISTRATION_ENTRIES {
            return None;
        }
        Some(Route {
            api: service.api.clone(),
        })
    }

    type DeleteBody = common_admin_api::delete_registration_entries::Request;
    async fn delete(self, body: Option<Self::DeleteBody>) -> http_common::server::RouteResponse {
        let body = body.ok_or_else(|| http_common::server::Error {
            status_code: http::StatusCode::BAD_REQUEST,
            message: "missing request body".into(),
        })?;

        let mut api = self.api.lock().await;
        let api = &mut *api;
        let res = api.delete_registration_entries(body).await;

        let res = http_common::server::response::json(hyper::StatusCode::OK, &res);

        Ok(res)
    }

    type PostBody = common_admin_api::create_registration_entries::Request;
    async fn post(self, body: Option<Self::PostBody>) -> http_common::server::RouteResponse {
        let body = body.ok_or_else(|| http_common::server::Error {
            status_code: http::StatusCode::BAD_REQUEST,
            message: "missing request body".into(),
        })?;

        let mut api = self.api.lock().await;
        let api = &mut *api;
        let res = api.create_registration_entries(body).await;

        let res = http_common::server::response::json(hyper::StatusCode::OK, &res);

        Ok(res)
    }

    type PutBody = serde::de::IgnoredAny;
}
