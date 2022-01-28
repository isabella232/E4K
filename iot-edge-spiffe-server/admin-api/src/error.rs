// Copyright (c) Microsoft. All rights reserved.

#[derive(Debug)]
pub enum Error {
    CatalogError(String),
    InvalidArguments(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::CatalogError(message) | Error::InvalidArguments(message) => f.write_str(message),
        }
    }
}
