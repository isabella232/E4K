// Copyright (c) Microsoft. All rights reserved.
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Unable to get key from catalog {0}")]
    CatalogGetKeys(Box<dyn std::error::Error + Send>),
}
