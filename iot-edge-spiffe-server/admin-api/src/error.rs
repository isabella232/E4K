// Copyright (c) Microsoft. All rights reserved.

use server_admin_api::operation;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Cannot list entries: {0}")]
    ListEntry(#[from] Box<dyn std::error::Error>),
    #[error("Error while creating entry: {0}")]
    CreateEntry(Box<dyn std::error::Error>, String),
    #[error("Error while deleting entry: {0}")]
    DeleteEntry(Box<dyn std::error::Error>, String),
    #[error("Error while getting entry: {0}")]
    GetEntry(Box<dyn std::error::Error>, String),
    #[error("Error while updating entry: {0}")]
    UpdateEntry(Box<dyn std::error::Error>, String),
    #[error("Invalid page size {0}")]
    InvalidPageSize(Box<dyn std::error::Error>),
    #[error("{0}")]
    InvalidArguments(Box<dyn std::error::Error>, String),
}

impl From<Error> for operation::Error {
    fn from(error: Error) -> Self {
        match &error {
            Error::CreateEntry(inner_error, id)
            | Error::GetEntry(inner_error, id)
            | Error::UpdateEntry(inner_error, id)
            | Error::DeleteEntry(inner_error, id) => Self {
                id: id.clone(),
                error: format!("{}", inner_error),
            },
            _ => Self {
                id: String::new(),
                error: "unsuported error".to_string(),
            },
        }
    }
}
