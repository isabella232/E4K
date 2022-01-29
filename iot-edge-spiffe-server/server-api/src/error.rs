// Copyright (c) Microsoft. All rights reserved.
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("{0}")]
    DummyError(String),
}
