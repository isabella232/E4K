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

pub struct SVID_signer {
    ttl: i64,
}

impl SVID_signer {
    pub fn sign_jwt_svid(spiffe_id: &SPIFFEID, audiences: &Vec<u8>) -> String {
        
    }

}