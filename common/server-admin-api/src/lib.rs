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

use std::{fmt, str};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ApiVersion {
    V2022_06_01,
}

impl fmt::Display for ApiVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            ApiVersion::V2022_06_01 => "2022-06-01",
        })
    }
}

impl str::FromStr for ApiVersion {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "2022-06-01" => Ok(ApiVersion::V2022_06_01),
            _ => Err(()),
        }
    }
}

pub mod create_registration_entries {
    use crate::{operation, RegistrationEntry};

    #[derive(Debug, serde::Deserialize, serde::Serialize)]
    pub struct Request {
        pub entries: Vec<RegistrationEntry>,
    }

    #[derive(Debug, serde::Deserialize, serde::Serialize)]
    pub struct Response {
        pub results: Vec<Result<String, operation::Error>>,
    }
}

pub mod update_registration_entries {
    use crate::{operation, RegistrationEntry};

    #[derive(Debug, serde::Deserialize, serde::Serialize)]
    pub struct Request {
        pub entries: Vec<RegistrationEntry>,
    }

    #[derive(Debug, serde::Deserialize, serde::Serialize)]
    pub struct Response {
        pub results: Vec<Result<String, operation::Error>>,
    }
}

pub mod list_registration_entries {
    use crate::RegistrationEntry;

    pub struct Params {
        pub page_size: u32,
        pub page_token: Option<String>,
    }

    #[derive(Debug, serde::Deserialize, serde::Serialize)]
    pub struct Response {
        pub entries: Vec<RegistrationEntry>,
        pub next_page_token: Option<String>,
    }
}

pub mod select_get_registration_entries {
    use crate::{operation, RegistrationEntry};

    #[derive(Debug, serde::Deserialize, serde::Serialize)]
    pub struct Request {
        pub ids: Vec<String>,
    }

    #[derive(Debug, serde::Deserialize, serde::Serialize)]
    pub struct Response {
        pub results: Vec<Result<RegistrationEntry, operation::Error>>,
    }
}

pub mod delete_registration_entries {
    use crate::operation;

    #[derive(Debug, serde::Deserialize, serde::Serialize)]
    pub struct Request {
        pub ids: Vec<String>,
    }

    #[derive(Debug, serde::Deserialize, serde::Serialize)]
    pub struct Response {
        pub results: Vec<Result<String, operation::Error>>,
    }
}

pub mod operation {
    #[derive(Debug, serde::Deserialize, serde::Serialize)]
    pub struct Error {
        pub id: String,
        pub error: String,
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct RegistrationEntry {
    pub id: String,
    pub iot_hub_id: Option<IoTHubId>,
    pub spiffe_id: String,
    pub parent_id: Option<String>,
    pub selectors: Vec<String>,
    pub admin: bool,
    pub ttl: u64,
    pub expires_at: u64,
    pub dns_names: Vec<String>,
    pub revision_number: u64,
    pub store_svid: bool,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct IoTHubId {
    pub iot_hub_hostname: String,
    pub device_id: String,
    pub module_id: String,
}
