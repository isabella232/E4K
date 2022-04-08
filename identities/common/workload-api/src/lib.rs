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

pub mod google {
    pub mod protobuf {
        tonic::include_proto!("google.protobuf");
    }
}

pub mod generated {
    #![allow(
        clippy::doc_markdown,
        clippy::must_use_candidate,
        clippy::wildcard_imports
    )]

    use crate::google;

    // NOTE: workload.proto is without a "package" directive, so the generated code is in "${OUT_DIR}/_.rs"
    tonic::include_proto!("_");
}

use generated::{
    spiffe_workload_api_client::SpiffeWorkloadApiClient, JwtBundlesRequest, JwtBundlesResponse,
    JwtsvidRequest, JwtsvidResponse, ValidateJwtsvidRequest, ValidateJwtsvidResponse,
};

#[cfg_attr(feature = "tests", mockall::automock)]
#[async_trait::async_trait]
pub trait WorkloadAPIClient: Send {
    async fn fetch_jwtsvid(
        &mut self,
        request: JwtsvidRequest,
    ) -> Result<tonic::Response<JwtsvidResponse>, tonic::Status>;

    async fn fetch_jwt_bundles(
        &mut self,
        request: JwtBundlesRequest,
    ) -> Result<tonic::Response<tonic::codec::Streaming<JwtBundlesResponse>>, tonic::Status>;

    async fn validate_jwtsvid(
        &mut self,
        request: ValidateJwtsvidRequest,
    ) -> Result<tonic::Response<ValidateJwtsvidResponse>, tonic::Status>;
}

#[async_trait::async_trait]
impl WorkloadAPIClient for SpiffeWorkloadApiClient<tonic::transport::Channel> {
    async fn fetch_jwtsvid(
        &mut self,
        request: JwtsvidRequest,
    ) -> Result<tonic::Response<JwtsvidResponse>, tonic::Status> {
        self.fetch_jwtsvid(request).await
    }

    async fn fetch_jwt_bundles(
        &mut self,
        request: JwtBundlesRequest,
    ) -> Result<tonic::Response<tonic::codec::Streaming<JwtBundlesResponse>>, tonic::Status> {
        self.fetch_jwt_bundles(request).await
    }

    async fn validate_jwtsvid(
        &mut self,
        request: ValidateJwtsvidRequest,
    ) -> Result<tonic::Response<ValidateJwtsvidResponse>, tonic::Status> {
        self.validate_jwtsvid(request).await
    }
}
