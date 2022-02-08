// Copyright (c) Microsoft. All rights reserved.

use crate::Api;
use catalog::Entries;
use http_common::make_service;
use server_admin_api::ApiVersion;

mod create_get_update_delete_entries;
mod get_select_entries;

pub struct Service<C>
where
    C: Entries + Send + Sync + 'static,
{
    pub(crate) api: Api<C>,
}

impl<C> Clone for Service<C>
where
    C: Entries + Send + Sync + 'static,
{
    fn clone(&self) -> Self {
        Self {
            api: self.api.clone(),
        }
    }
}

make_service! {
    service: Service<C>,
    {<C: 'static>}
    {C: Entries + Send + Sync + 'static}
    api_version: ApiVersion,
    routes: [
        create_get_update_delete_entries::Route<C>,
        get_select_entries::Route<C>,
    ],
}
