// Copyright (c) Microsoft. All rights reserved.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Error while trying the get trust bundle from server {0}")]
    TrustBundleResponse(Box<dyn std::error::Error + Send>),
    #[error("Error while trying to convert the trust jwkset to vec<u8> {0}")]
    SerdeConvertToVec(serde_json::Error),
}

impl From<Error> for tonic::Status {
    fn from(error: Error) -> Self {
        tonic::Status::unknown(format!("{}", error))
    }
}
