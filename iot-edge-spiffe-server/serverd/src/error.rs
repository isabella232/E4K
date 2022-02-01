// Copyright (c) Microsoft. All rights reserved.
// Copyright (c) Microsoft. All rights reserved.
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Error parsing config {0}")]
    ErrorParsingConfig(std::io::Error),
}
