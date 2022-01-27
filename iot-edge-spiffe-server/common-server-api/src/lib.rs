// Copyright (c) Microsoft. All rights reserved.

#![deny(rust_2018_idioms)]
#![warn(clippy::all, clippy::pedantic)]
#![allow(
    clippy::default_trait_access,
    clippy::let_and_return,
    clippy::let_unit_value,
    clippy::missing_errors_doc,
    clippy::similar_names,
    clippy::too_many_lines
)]

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ApiVersion {
    V2020_09_01,
}

impl std::fmt::Display for ApiVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            ApiVersion::V2020_09_01 => "2020-09-01",
        })
    }
}

impl std::str::FromStr for ApiVersion {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "2020-09-01" => Ok(ApiVersion::V2020_09_01),
            _ => Err(()),
        }
    }
}

pub mod create_new_jwt {
    use crate::JWTSVID;

    #[derive(Debug, serde::Deserialize, serde::Serialize)]
    pub struct Request {
        pub id: String,
        pub audiences: Vec<String>,
    }

    #[derive(Debug, serde::Deserialize, serde::Serialize)]
    pub struct Response {
        pub jwt_svid: JWTSVID,
    }
}

pub mod get_trust_bundle {
    use crate::{Bundle, Settings};

    #[derive(Debug, serde::Deserialize, serde::Serialize)]
    pub struct Request {
        pub id: Settings,
    }

    #[derive(Debug, serde::Deserialize, serde::Serialize)]
    pub struct Response {
        pub bundle: Bundle,
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct JWTSVID {
    pub token: String,
    pub spiffe_id: SPIFFEID,
    pub expire_at: i64,
    pub issued_at: i64,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct SPIFFEID {
    pub trust_domain: String,
    pub path: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct Settings {
    pub jwt_keys: bool,
    pub x509_cas: bool,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct Bundle {
    pub trust_domain: String,
    pub jwt_keys: Option<Vec<JWK>>,
    pub x509_cas: Option<Vec<Vec<u8>>>,
    pub refresh_hint: i64,
    pub sequence_number: i64,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct JWK {
    pub public_key: Vec<u8>,
    pub key_id: String,
    pub expires_at: i64,
}
