// Copyright (c) Microsoft. All rights reserved.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Cannot list entries: {0}")]
    ListEntry(#[from] Box<dyn std::error::Error>),
    #[error("Invalid page size {0}")]
    InvalidPageSize(Box<dyn std::error::Error>),
}
