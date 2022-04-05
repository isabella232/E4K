// Copyright (c) Microsoft. All rights reserved.

use core_objects::AttestationConfig;
use core_objects::IdentityTypes;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Config {
    pub trust_domain: String,
    pub server_socket_path: String,
    pub provisioning: Provisioning,
    pub entry: Vec<Entry>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Provisioning {
    pub iothub_hostname: Option<String>,
    pub device_id: Option<String>,
    pub auth: AuthMethod,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "method", rename_all = "UPPERCASE")]
pub enum AuthMethod {
    X509(X509Auth),
    Sas(SASAuth),
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct X509Auth {
    pub identity_pk: String,
    pub identity_cert: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SASAuth {
    pub connection_string: String,
}

impl Default for AuthMethod {
    fn default() -> Self {
        Self::X509(Default::default())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    pub spiffe_id: String,
    pub attestation_config: AttestationConfig,
    #[serde(default)]
    pub other_identities: Vec<(IdentityTypes, String)>,
    pub parent_id: Option<String>,
}

#[cfg(test)]
mod tests {
    use std::{fs::File, io::Read};

    use super::*;

    #[test]
    fn test_read_all() {
        let test_files_directory =
            std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/src/test_deployments"));

        for test_file in std::fs::read_dir(test_files_directory).unwrap() {
            let test_file = test_file.unwrap();
            if test_file.file_type().unwrap().is_dir() {
                continue;
            }
            let test_file = test_file.path();

            println!("Parsing deployment file {:#?}", test_file);
            let mut raw_config = File::open(&test_file).unwrap();
            let mut buf = Vec::new();
            raw_config.read_to_end(&mut buf).unwrap();

            let _config: Config = toml::from_slice(&buf).unwrap_or_else(|err| {
                panic!(
                    "Could not parse deployment file {:#?}, error {}",
                    test_file, err
                )
            });
        }
    }
}
