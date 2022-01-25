// Copyright (c) Microsoft. All rights reserved.

pub enum Error {
    CatalogError(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::CatalogError(message) => f.write_str(message),
        }
    }
}
