// Copyright (c) Microsoft. All rights reserved.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("{0}")]
    DuplicatedEntry(String),
    #[error("{0}")]
    EntryDoNotExist(String),
    #[error("{0}")]
    InvalidArguments(String),
}
