// Copyright (c) Microsoft. All rights reserved.
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Unable to create new JWT-SVID {0}")]
    CreateJWT(svid_factory::error::Error),
    #[error("Unable to build the trust bundle {0}")]
    BuildTrustBundle(trust_bundle_builder::error::Error),
    #[error("Catalog responded with an invalid number of entries")]
    InvalidResponse,
    #[error("Unable to get entry from catalog {0}")]
    CatalogGetEntry(Box<dyn std::error::Error + Send>),
}
