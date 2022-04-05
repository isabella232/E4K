// Copyright (c) Microsoft. All rights reserved.

#![deny(rust_2018_idioms)]
#![warn(clippy::all, clippy::pedantic)]
#![allow(
    clippy::default_trait_access,
    clippy::let_unit_value,
    clippy::missing_errors_doc,
    clippy::similar_names,
    clippy::too_many_lines
)]
pub mod error;
pub mod validate;

#[cfg(feature = "tests")]
use mockall::automock;

use core_objects::{TrustBundle, JWTSVID};
use error::Error;

// Put behind a trait, mainly for mocking.
#[cfg_attr(feature = "tests", automock)]
#[async_trait::async_trait]
pub trait JWTSVIDValidator: Send + Sync {
    async fn validate(
        &self,
        jwt_svid_compact: &str,
        trust_bundle: &TrustBundle,
        audience: &str,
    ) -> Result<JWTSVID, Error>;
}
