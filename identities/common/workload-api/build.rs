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

use tokio::fs;

#[tokio::main]
async fn main() {
    let response = reqwest::get("https://raw.githubusercontent.com/spiffe/go-spiffe/v1.1.0/proto/spiffe/workload/workload.proto".to_string())
    .await
    .unwrap()
    .bytes()
    .await
    .unwrap();
    fs::File::create("workloadapi.proto").await.unwrap();
    fs::write("workloadapi.proto", response).await.unwrap();

    tonic_build::compile_protos("./workloadapi.proto").unwrap();
}
