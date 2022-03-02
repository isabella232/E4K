// Copyright (c) Microsoft. All rights reserved.

use std::io;

use http_common::ConnectorError;
use thiserror::Error;
use url::ParseError;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Could not parse server address {0}")]
    InvalidAddress(ParseError),
    #[error("Could create connector with given address {0}")]
    Connector(String),
    #[error("Error while doing agent attestation with server {0}")]
    AttestAgent(io::Error),
    #[error("Error while deserializing response from attest agent request {0}")]
    DeserializingAttestAgentResponse(io::Error),
}

impl From<ConnectorError> for Error {
    fn from(err: ConnectorError) -> Self {
        Error::Connector(format!("{}", err))
    }
}
