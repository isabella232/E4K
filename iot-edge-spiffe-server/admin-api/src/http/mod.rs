// Copyright (c) Microsoft. All rights reserved.

use crate::Api;
use catalog::Catalog;
use http_common::make_service;
use server_admin_api::ApiVersion;

mod create_get_update_delete_entries;
mod get_select_entries;

pub struct Service<C: Catalog + Send + Sync> {
    pub(crate) api: Api<C>,
}

impl<C> Clone for Service<C>
where
    C: Catalog + Send + Sync,
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
    {C:Catalog + Sync + Send}
    api_version: ApiVersion,
    routes: [
        create_get_update_delete_entries::Route<C>,
        get_select_entries::Route<C>,
    ],
}
