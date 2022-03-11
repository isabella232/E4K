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

use std::{
    collections::{BTreeMap, BTreeSet},
    time::SystemTime,
};

use serde::{Deserialize, Serialize};

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone, PartialEq)]
pub struct SPIFFEID {
    pub trust_domain: String,
    pub path: String,
}

impl std::fmt::Display for SPIFFEID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.trust_domain, self.path)
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct RegistrationEntry {
    pub id: String,
    pub other_identities: Vec<IdentityTypes>,
    pub spiffe_id: SPIFFEID,
    pub attestation_config: AttestationConfig,
    pub admin: bool,
    pub ttl: u64,
    pub expires_at: u64,
    pub dns_names: Vec<String>,
    pub revision_number: u64,
    pub store_svid: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "content", rename_all = "UPPERCASE")]
pub enum AttestationConfig {
    Workload(EntryWorkloadAttestation),
    Node(EntryNodeAttestation),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EntryWorkloadAttestation {
    pub parent_id: String,
    pub value: Vec<WorkloadSelector>,
    pub plugin: WorkloadAttestationPlugin,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EntryNodeAttestation {
    pub value: Vec<NodeSelector>,
    pub plugin: NodeAttestationPlugin,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "content", rename_all = "UPPERCASE")]
pub enum NodeSelector {
    Cluster(String),
    AgentNameSpace(String),
    AgentServiceAccount(String),
    AgentPodName(String),
    AgentPodUID(String),
    AgentNodeIP(String),
    AgentNodeName(String),
    AgentNodeUID(String),
    AgentNodeLabels(BTreeMap<String, String>),
    AgentPodLabels(BTreeMap<String, String>),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "content", rename_all = "UPPERCASE")]
pub enum WorkloadSelector {
    NameSpace(String),
    ServiceAccount(String),
    PodName(String),
    PodUID(String),
    NodeName(String),
    PodLabels(BTreeMap<String, String>),
    ContainerName(String),
    ContainerImage(String),
    PodOwners(BTreeSet<String>),
    PodOwnerUIDs(BTreeSet<String>),
    PodImages(BTreeSet<String>),
    PodImageCount(usize),
    PodInitImages(BTreeSet<String>),
    PodInitImageCount(usize),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Ord, PartialOrd)]
pub enum WorkloadSelectorType {
    NameSpace,
    ServiceAccount,
    PodName,
    PodUID,
    NodeName,
    PodLabels,
    ContainerName,
    ContainerImage,
    ContainerImageId,
    PodOwners,
    PodOwnerUIDs,
    PodImages,
    PodImageCount,
    PodInitImages,
    PodInitImageCount,
}

impl From<&WorkloadSelector> for WorkloadSelectorType {
    fn from(selector: &WorkloadSelector) -> Self {
        match selector {
            WorkloadSelector::NameSpace(_) => WorkloadSelectorType::NameSpace,
            WorkloadSelector::ServiceAccount(_) => WorkloadSelectorType::ServiceAccount,
            WorkloadSelector::PodName(_) => WorkloadSelectorType::PodName,
            WorkloadSelector::PodUID(_) => WorkloadSelectorType::PodUID,
            WorkloadSelector::NodeName(_) => WorkloadSelectorType::NodeName,
            WorkloadSelector::PodLabels(_) => WorkloadSelectorType::PodLabels,
            WorkloadSelector::ContainerName(_) => WorkloadSelectorType::ContainerName,
            WorkloadSelector::ContainerImage(_) => WorkloadSelectorType::ContainerImage,
            WorkloadSelector::PodOwners(_) => WorkloadSelectorType::PodOwners,
            WorkloadSelector::PodOwnerUIDs(_) => WorkloadSelectorType::PodOwnerUIDs,
            WorkloadSelector::PodImages(_) => WorkloadSelectorType::PodImages,
            WorkloadSelector::PodImageCount(_) => WorkloadSelectorType::PodImageCount,
            WorkloadSelector::PodInitImages(_) => WorkloadSelectorType::PodInitImages,
            WorkloadSelector::PodInitImageCount(_) => WorkloadSelectorType::PodInitImageCount,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum NodeAttestationPlugin {
    Psat,
    Sat,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum WorkloadAttestationPlugin {
    K8s,
    Docker,
}

#[derive(PartialEq, Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct IoTHubId {
    pub iot_hub_hostname: String,
    pub device_id: String,
    pub module_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JWTSVID {
    pub header: JWTHeader,
    pub claims: JWTClaims,
    pub signature: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JWTHeader {
    pub algorithm: KeyType,
    pub key_id: String,
    pub jwt_type: JWTType,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JWTClaims {
    pub subject: SPIFFEID,
    pub audience: Vec<SPIFFEID>,
    pub expiry: u64,
    pub issued_at: u64,
    pub other_identities: Vec<IdentityTypes>,
}

#[derive(PartialEq, Debug, serde::Deserialize, serde::Serialize, Clone)]
#[serde(tag = "type", content = "content", rename_all = "UPPERCASE")]
pub enum IdentityTypes {
    IoTHub(IoTHubId),
    Custom(String),
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub enum JWTType {
    JWT,
    JOSE,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
pub enum KeyType {
    RS256,
    RS384,
    RS512,
    ES256,
    ES384,
    ES512,
    PS256,
    PS384,
    PS512,
}

impl From<KeyType> for (Kty, Crv) {
    fn from(key_type: KeyType) -> (Kty, Crv) {
        match key_type {
            KeyType::ES256 => (Kty::EC, Crv::P256),
            KeyType::ES384 => (Kty::EC, Crv::P384),
            KeyType::ES512 => (Kty::EC, Crv::P521),
            _ => unimplemented!(),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
pub enum KeyUse {
    #[serde(rename = "x509-svid")]
    X509SVID,
    #[serde(rename = "jwt-svid")]
    JWTSVID,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone, PartialEq)]
pub struct JWTSVIDCompact {
    pub token: String,
    pub spiffe_id: SPIFFEID,
    pub expiry: u64,
    pub issued_at: u64,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct TrustBundle {
    pub trust_domain: String,
    pub jwt_key_set: JWKSet,
    pub x509_key_set: JWKSet,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone, PartialEq)]
pub struct JWKSet {
    pub keys: Vec<JWK>,
    pub spiffe_refresh_hint: u64,
    pub spiffe_sequence_number: u64,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone, PartialEq)]
pub struct JWK {
    pub x: String,
    pub y: String,
    pub kty: Kty,
    pub crv: Crv,
    pub kid: String,
    #[serde(rename = "use")]
    pub key_use: KeyUse,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone, PartialEq)]
pub enum Kty {
    EC,
    RSA,
    #[serde(rename = "oct")]
    Oct,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone, PartialEq)]
pub enum Crv {
    #[serde(rename = "P-256")]
    P256,
    #[serde(rename = "P-384")]
    P384,
    #[serde(rename = "P-521")]
    P521,
}

#[must_use]
pub fn get_epoch_time() -> u64 {
    let now = SystemTime::now();
    let epoch = now
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("Epoch should succeed");
    epoch.as_secs()
}

#[cfg(feature = "tests")]
pub const CONFIG_DEFAULT_PATH: &str = "../../iot-edge-spiffe-server/config/tests/Config.toml";

#[cfg(feature = "tests")]
pub const AGENT_DEFAULT_CONFIG_PATH: &str = "../../iot-edge-spiffe-agent/config/tests/Config.toml";
