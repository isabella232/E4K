// Copyright (c) Microsoft. All rights reserved.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Unable to read the Service Account Token {0}")]
    UnableToReadToken(std::io::Error),
}
