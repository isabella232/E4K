// Copyright (c) Microsoft. All rights reserved.

#[derive(Debug)]
pub enum Error {
    ErrorParsingConfig(std::io::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::ErrorParsingConfig(_) => f.write_str("Error parsing config"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::ErrorParsingConfig(err) => Some(err),
        }
    }
}
