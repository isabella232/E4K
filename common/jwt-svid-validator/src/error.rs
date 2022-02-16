// Copyright (c) Microsoft. All rights reserved.

use std::str::Utf8Error;

use base64::DecodeError;
use core_objects::{JWTType, KeyType, SPIFFEID};
use openssl::error::ErrorStack;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Expected 3 parts separated by '.', found: {0}")]
    InvalidJoseEncoding(usize),
    #[error("Unable to deserialize Json: {0}")]
    DeserializeJson(serde_json::Error),
    #[error("Invalid header algorithm: {0:?}")]
    InvalidAlgorithm(KeyType),
    #[error("Invalid header jwt type: {0:?}")]
    InvalidJWTType(JWTType),
    #[error("Error decoding from base64: {0}")]
    InvalidBase64Encoding(DecodeError),
    #[error("Error decoding from base64: {0}")]
    InvalidUTF8Encoding(Utf8Error),
    #[error("Token is expired: current time {current:?}, expiry time {current:?}")]
    ExpiredToken { expiry: u64, current: u64 },
    #[error("Identity {0:?} is not in audience field")]
    InvalidAudience(SPIFFEID),
    #[error("The trust bundle is not issued for the trust domain: Expected {expected:?}, found {found:?}")]
    InvalidTrustDomain { expected: String, found: String },
    #[error("Could not find public key kid: ")]
    PublicKeyNotInTrustBundle(String),
    #[error(
        "Public key in trust bundle is expired: current time {current:?}, expiry time {current:?}"
    )]
    ExpiredPublicKey { expiry: u64, current: u64 },
    #[error("Cannot convert public key der to openssl public key: {0}")]
    CannotConvertDerToEcdsaPublicKey(ErrorStack),
    #[error("Cannot convert pulic key der to ecdsa public key: {0}")]
    CannotConvertSignatureToEcdsaSignature(ErrorStack),
    #[error("Error while verifying the signature: {0}")]
    SignatureVerificationErrorEcdsa(ErrorStack),
    #[error("The signature doesn't match the expected signature")]
    InvalidSignature,
}
