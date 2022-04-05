// Copyright (c) Microsoft. All rights reserved.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Could not get the trust bundle during initialization")]
    InitTrustBundle(Box<dyn std::error::Error + Send>),
    #[error("Could not refresh the trust bundle")]
    TrustBundle(Box<dyn std::error::Error + Send>),
}
