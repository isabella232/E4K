// Copyright (c) Microsoft. All rights reserved.

use catalog::{Entries, TrustBundleStore};

use http_common::make_service;
use key_store::KeyStore;
use server_agent_api::ApiVersion;

use crate::Api;

mod create_new_jwt;
mod get_trust_bundle;

pub struct Service<C, D>
where
    C: Entries + TrustBundleStore + Send + Sync + 'static,
    D: KeyStore + Send + Sync + 'static,
{
    pub(crate) api: Api<C, D>,
}

impl<C, D> Clone for Service<C, D>
where
    C: Entries + TrustBundleStore + Send + Sync + 'static,
    D: KeyStore + Send + Sync + 'static,
{
    fn clone(&self) -> Self {
        Self {
            api: self.api.clone(),
        }
    }
}

make_service! {
    service: Service<C, D>,
    {<C: 'static, D: 'static>}
    {C: Entries + TrustBundleStore + Send + Sync + 'static, D: KeyStore + Send + Sync + 'static}
    api_version: ApiVersion,
    routes: [
        create_new_jwt::Route<C, D>,
        get_trust_bundle::Route<C, D>,
    ],
}
