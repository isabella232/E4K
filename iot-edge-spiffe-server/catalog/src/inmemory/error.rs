// Copyright (c) Microsoft. All rights reserved.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Entry {0} already exist")]
    DuplicatedEntry(String),
    #[error("Entry {0} do not exists")]
    EntryNotFound(String),
    #[error("Invalid page size")]
    InvalidPageSize(),
}
