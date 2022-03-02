// Copyright (c) Microsoft. All rights reserved.

use crate::Api;
use http_common::make_service;
use server_admin_api::ApiVersion;

mod create_get_update_delete_entries;
mod get_select_entries;

#[derive(Clone)]
pub struct Service {
    pub(crate) api: Api,
}

make_service! {
    service: Service,
    api_version: ApiVersion,
    routes: [
        create_get_update_delete_entries::Route,
        get_select_entries::Route,
    ],
}

pub mod uri {
    pub const CREATE_DELETE_UPDATE_REGISTRATION_ENTRIES: &str = "/entries";
    pub const SELECT_GET_REGISTRATION_ENTRIES: &str = "/select-list-entries";
}
