// Copyright (c) Microsoft. All rights reserved.
mod entries;
mod error;
mod trust_bundle_store;

use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

use openssl::pkey::{PKey, Public};
use parking_lot::{const_mutex, Mutex};
use server_admin_api::RegistrationEntry;

use self::error::Error;

pub struct Catalog {
    entries_list: Arc<Mutex<BTreeMap<String, RegistrationEntry>>>,
    // Since this is in memory implementation, there is only one trust domain
    // The trust domain string will be ignored in the calls related to the trust domain key store
    // That one hashmap contains all the public keys for the only trust domain.
    jwt_trust_domain_store: Arc<Mutex<HashMap<String, PKey<Public>>>>,
}

impl Catalog {
    #[must_use]
    pub fn new() -> Self {
        Catalog {
            entries_list: Arc::new(const_mutex(BTreeMap::new())),
            jwt_trust_domain_store: Arc::new(const_mutex(HashMap::new())),
        }
    }
}

impl Default for Catalog {
    fn default() -> Self {
        Self::new()
    }
}
