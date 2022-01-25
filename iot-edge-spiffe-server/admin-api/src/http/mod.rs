// Copyright (c) Microsoft. All rights reserved.

mod create_delete_registration_entries;
mod list_registration_entries;
mod select_list_registration_entries;

#[derive(Clone)]
pub struct Service {
    pub(crate) api: std::sync::Arc<futures_util::lock::Mutex<crate::Api>>,
}

http_common::make_service! {
    service: Service,
    api_version: common_admin_api::ApiVersion,
    routes: [
        create_delete_registration_entries::Route,
        list_registration_entries::Route,
        select_list_registration_entries::Route,
    ],
}
