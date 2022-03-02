// Copyright (c) Microsoft. All rights reserved.

use http_common::make_service;
use server_agent_api::ApiVersion;

use crate::Api;

mod attest_agent;
mod create_workload_jwt;
mod get_trust_bundle;

#[derive(Clone)]
pub struct Service {
    pub(crate) api: Api,
}

pub mod uri {
    pub const CREATE_WORKLOAD_JTW: &str = "/workload-jwt";
    pub const ATTEST_AGENT: &str = "/attest-agent";
    pub const GET_TRUST_BUNDLE: &str = "/trust-bundle";
}

make_service! {
    service: Service,
    api_version: ApiVersion,
    routes: [
        create_workload_jwt::Route,
        attest_agent::Route,
        get_trust_bundle::Route,
    ],
}
