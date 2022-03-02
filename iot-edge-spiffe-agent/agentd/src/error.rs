// Copyright (c) Microsoft. All rights reserved.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Error parsing config {0}")]
    ParsingConfig(std::io::Error),
    #[error("Error Creating server client {0}")]
    CreatingServerclient(Box<dyn std::error::Error + Send>),
}
