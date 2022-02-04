// Copyright (c) Microsoft. All rights reserved.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Entry {0} already exist")]
    DuplicatedEntry(String),
    #[error("Entry {0} do not exists")]
    EntryNotFound(String),
    #[error("Key {0} already exist")]
    DuplicatedKey(String),
    #[error("Key {0} do not exists")]
    KeyNotFound(String),
    #[error("Invalid page size")]
    InvalidPageSize(),
}
