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

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ApiVersion {
    V2022_06_01,
}

impl std::fmt::Display for ApiVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            ApiVersion::V2022_06_01 => "2022-06-01",
        })
    }
}

impl std::str::FromStr for ApiVersion {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "2022-06-01" => Ok(ApiVersion::V2022_06_01),
            _ => Err(()),
        }
    }
}

pub mod attest_agent {
    use core_objects::JWTSVIDCompact;

    #[derive(Debug, serde::Deserialize, serde::Serialize)]
    pub struct Auth {
        pub token: String,
    }

    #[derive(Debug, serde::Deserialize, serde::Serialize)]
    pub struct Response {
        pub jwt_svid: JWTSVIDCompact,
    }
}

pub mod create_workload_jwt {
    use core_objects::{JWTSVIDCompact, SPIFFEID};

    #[derive(Debug, serde::Deserialize, serde::Serialize)]
    pub struct Request {
        pub id: String,
        pub audiences: Vec<SPIFFEID>,
    }

    #[derive(Debug, serde::Deserialize, serde::Serialize)]
    pub struct Response {
        pub jwt_svid: JWTSVIDCompact,
    }
}

pub mod get_trust_bundle {
    use core_objects::TrustBundle;

    pub struct Params {
        pub jwt_keys: bool,
        pub x509_cas: bool,
    }

    #[derive(Debug, serde::Deserialize, serde::Serialize)]
    pub struct Response {
        pub trust_bundle: TrustBundle,
    }
}
