// Copyright (c) Microsoft. All rights reserved.

pub enum Error {
    DuplicatedEntry(String),
    EntryDoNotExist(String),
    InvalidArguments(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::DuplicatedEntry(message)
            | Error::EntryDoNotExist(message)
            | Error::InvalidArguments(message) => f.write_str(message),
        }
    }
}
