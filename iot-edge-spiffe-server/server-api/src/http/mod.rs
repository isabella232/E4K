// Copyright (c) Microsoft. All rights reserved.
use common_server_api::ApiVersion;
use http_common::make_service;

use crate::Api;

mod create_new_jwt;
mod get_trust_bundle;

#[derive(Clone)]
pub struct Service {
    pub(crate) api: Api,
}

make_service! {
    service: Service,
    api_version: ApiVersion,
    routes: [
        create_new_jwt::Route,
        get_trust_bundle::Route,
    ],
}
