// Copyright (c) Microsoft. All rights reserved.
mod entries;
mod error;
mod trust_bundle_store;

use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

use crate::Catalog as CatalogTrait;
use core_objects::{RegistrationEntry, JWK};
use parking_lot::{const_rwlock, RwLock};

pub struct Catalog {
    entries_list: Arc<RwLock<BTreeMap<String, RegistrationEntry>>>,
    jwt_trust_domain: Arc<RwLock<JWTTrustDomain>>,
}

pub struct JWTTrustDomain {
    version: usize,
    // Since this is in memory implementation, there is only one trust domain
    // The trust domain string will be ignored in the calls related to the trust domain key store
    // That one hashmap contains all the public keys for the only trust domain.
    store: HashMap<String, JWK>,
}

impl Catalog {
    #[must_use]
    pub fn new() -> Self {
        Catalog {
            entries_list: Arc::new(const_rwlock(BTreeMap::new())),
            jwt_trust_domain: Arc::new(const_rwlock(JWTTrustDomain {
                version: 0,
                store: HashMap::new(),
            })),
        }
    }
}

impl Default for Catalog {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl CatalogTrait for Catalog {}
