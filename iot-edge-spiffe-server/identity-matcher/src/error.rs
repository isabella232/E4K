// Copyright (c) Microsoft. All rights reserved.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Could not iterate of entries in catalog {0}")]
    CatalogGetEntries(#[from] Box<dyn std::error::Error + Send>),
}
