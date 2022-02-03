// Copyright (c) Microsoft. All rights reserved.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Error while creating a new key {0}")]
    ErrorCreatingNewKey(#[from] Box<dyn std::error::Error>),
    #[error("Error while deleting the old key {0}")]
    ErrorDeletingOldKey(Box<dyn std::error::Error>),
    #[error("Tried to rotate but there is not next jwt key to replace the current one")]
    ErrorNextJwtKeyMissing(),
}
