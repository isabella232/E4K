// Copyright (c) Microsoft. All rights reserved.

mod create_new_jwt;
mod get_trust_bundle;

#[derive(Clone)]
pub struct Service {
    pub(crate) api: std::sync::Arc<futures_util::lock::Mutex<crate::Api>>,
}

http_common::make_service! {
    service: Service,
    api_version: common_server_api::ApiVersion,
    routes: [
        create_new_jwt::Route,
        get_trust_bundle::Route,
    ],
}
