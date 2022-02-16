// Copyright (c) Microsoft. All rights reserved.
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Error parsing config {0}")]
    ErrorJSONSerializing(serde_json::Error),
    #[error("Error while signing digest with current key {0}")]
    SigningDigest(Box<dyn std::error::Error + Send>),
}
