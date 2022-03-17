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

use core_objects::JWKSet;
use log::info;
use tokio::net::UnixStream;
use tonic::transport::{Endpoint, Uri};
use tower::service_fn;
use workload_api::{
    spiffe_workload_api_client::SpiffeWorkloadApiClient, JwtBundlesRequest, JwtsvidRequest,
    ValidateJwtsvidRequest,
};

#[tokio::main]
async fn main() {
    logger::try_init()
        .expect("cannot fail to initialize global logger from the process entrypoint");

    info!("Starting Workload API Test Client");
    // create a new Workload API client connecting to the provided endpoint socket path

    let channel = Endpoint::try_from("http://[::]:50051")
        .unwrap()
        .connect_with_connector(service_fn(|_: Uri| {
            let path = "/run/iotedge/sockets/workloadapi.sock";

            // Connect to a Uds socket
            UnixStream::connect(path)
        }))
        .await
        .unwrap();

    let mut client = SpiffeWorkloadApiClient::new(channel);

    let request = JwtBundlesRequest::default();
    let mut response = client.fetch_jwt_bundles(request).await.unwrap();
    let trust_bundle = response.get_mut().message().await.unwrap().unwrap();

    for (trust_domain, jwk_set) in trust_bundle.bundles {
        let jwk_set: JWKSet = serde_json::from_slice(&jwk_set).unwrap();
        info!(
            "Got trust bundle {:?} in trust domain {}",
            jwk_set, trust_domain
        );
    }

    let request = JwtsvidRequest {
        audience: vec!["dummy_audience".to_string()],
        spiffe_id: String::new(),
    };
    let response = client.fetch_jwtsvid(request).await.unwrap();
    let svids = response.into_inner().svids;
    info!("Got svids {:?}", svids);

    let request = ValidateJwtsvidRequest {
        audience: "dummy_audience".to_string(),
        svid: svids[0].svid.clone(),
    };
    let response = client.validate_jwtsvid(request).await.unwrap();
    let claims = response.into_inner();
    info!("Got claims {:?}", claims);
}
