// Copyright (c) Microsoft. All rights reserved.

use std::{io, num::TryFromIntError};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("key could not be found: {0}")]
    KeyNotFound(String),
    #[error("File not found: {0}")]
    FileReadError(io::Error),
    #[error("Failed to write to file system: {0}")]
    FileWrite(io::Error),
    #[error("Failed to delete key from file system: {0}")]
    FileDelete(io::Error),
    #[error("Openssl Error")]
    OpenSSL(),
    #[error("Failed to convert number to usize: {0}, {1}")]
    ConvertToUsize(TryFromIntError, String),    
    #[error("Invalid parameters")]
    InvalidParameters(String),
    #[error("Unsupported Key pair type")]
    UnsupportedKeyPairType(),
}

impl From<openssl::error::Error> for Error {
    fn from(err: openssl::error::Error) -> Self {
        log::error!("{}", err);
        Error::OpenSSL()
    }
}

impl From<openssl::error::ErrorStack> for Error {
    fn from(err: openssl::error::ErrorStack) -> Self {
        log::error!("{}", err);
        Error::OpenSSL()
    }
}

impl From<openssl2::Error> for Error {
    fn from(err: openssl2::Error) -> Self {
        log::error!("{}", err);
        Error::OpenSSL()
    }
}
