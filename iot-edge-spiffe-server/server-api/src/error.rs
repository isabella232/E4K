// Copyright (c) Microsoft. All rights reserved.
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Unable to create new workload JWT-SVID {0}")]
    CreateWorkloadJWT(svid_factory::error::Error),
    #[error("Unable to build the trust bundle {0}")]
    BuildTrustBundle(trust_bundle_builder::error::Error),
    #[error("Could not match identity {0}")]
    MatchIdentity(identity_matcher::error::Error),
    #[error("Unable to attest new agent {0}")]
    AttestAgent(Box<dyn std::error::Error + Send>),
}
