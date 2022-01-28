// Copyright (c) Microsoft. All rights reserved.

use crate::Api;
use common_admin_api::ApiVersion;
use http_common::make_service;

mod crud_entries;
mod select_get_registration_entries;

#[derive(Clone)]
pub struct Service {
    pub(crate) api: Api,
}

make_service! {
    service: Service,
    api_version: ApiVersion,
    routes: [
        crud_entries::Route,
        select_get_registration_entries::Route,
    ],
}
