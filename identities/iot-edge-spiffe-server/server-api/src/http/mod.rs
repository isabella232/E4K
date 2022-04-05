// Copyright (c) Microsoft. All rights reserved.

use http_common::make_service;
use server_agent_api::ApiVersion;

use crate::Api;

mod create_workload_jwts;
mod get_trust_bundle;

#[derive(Clone)]
pub struct Service {
    pub(crate) api: Api,
}

pub mod uri {
    pub const CREATE_WORKLOAD_JTWS: &str = "/workload-jwts";
    pub const GET_TRUST_BUNDLE: &str = "/trust-bundle";
}

make_service! {
    service: Service,
    api_version: ApiVersion,
    routes: [
        create_workload_jwts::Route,
        get_trust_bundle::Route,
    ],
}
