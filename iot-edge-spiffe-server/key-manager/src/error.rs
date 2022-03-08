// Copyright (c) Microsoft. All rights reserved.

use openssl::error::ErrorStack;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Error while creating a new private key {0}")]
    CreatingNewKey(#[from] Box<dyn std::error::Error>),
    #[error("Converting key to ec key {0}")]
    ECkeyConvertion(ErrorStack),
    #[error("Error creating big num object {0}")]
    BigNumGeneration(ErrorStack),
    #[error("Error while generating X and Y {0}")]
    GenerateXandY(ErrorStack),
    #[error("Error converting public key to raw {0}")]
    ConvertingKey(Box<dyn std::error::Error>),
    #[error("Error while deleting the old private key {0}")]
    DeletingPrivateKey(Box<dyn std::error::Error>),
    #[error("Error while deleting public key from catalog {0}")]
    DeletingPublicKey(Box<dyn std::error::Error>),
    #[error("Error while getting public for new key {0}")]
    GettingPulicKey(Box<dyn std::error::Error>),
    #[error("Error while adding public into the catalog {0}")]
    AddingPulicKey(Box<dyn std::error::Error>),
    #[error("Tried to rotate but there is not next jwt key to replace the current one")]
    NextJwtKeyMissing(),
}
