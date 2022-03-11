// Copyright (c) Microsoft. All rights reserved.

use std::num::TryFromIntError;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Error while trying the get trust bundle from server {0}")]
    TrustBundleResponse(Box<dyn std::error::Error + Send>),
    #[error("Error while trying to convert the trust jwkset to vec<u8> {0}")]
    SerdeConvertToVec(serde_json::Error),
    #[error("Could not get client PID from uds info")]
    UdsClientPID,
    #[error("Failed to get selectors from workload PID {0}")]
    WorkloadAttestation(Box<dyn std::error::Error + Send>),
    #[error("Process ID is negative {0}")]
    NegativePID(TryFromIntError),
    #[error("Failed to fetch new JWT-SVIDs for the workload {0}")]
    CreateJWTSVIDs(Box<dyn std::error::Error + Send>),
}

impl From<Error> for tonic::Status {
    fn from(error: Error) -> Self {
        tonic::Status::unknown(format!("{}", error))
    }
}
