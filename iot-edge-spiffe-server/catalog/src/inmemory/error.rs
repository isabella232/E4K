// Copyright (c) Microsoft. All rights reserved.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Entry {0} already exists")]
    DuplicatedEntry(String),
    #[error("Entry {0} does not exist")]
    EntryNotFound(String),
    #[error("Key {0} already exists")]
    DuplicatedKey(String),
    #[error("Key {0} does not exist")]
    KeyNotFound(String),
    #[error("Invalid page size")]
    InvalidPageSize(),
}
