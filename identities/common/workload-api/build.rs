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

fn main() {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let proto = std::path::Path::new(&out_dir).join("workload.proto");
    let status = std::process::Command::new("curl")
        .args([
            "--silent",
            "--show-error",
            "--proto",
            "=https",
            "--tlsv1.2",
            "--output",
            proto.to_str().unwrap(),
            "https://raw.githubusercontent.com/spiffe/go-spiffe/v1.1.0/proto/spiffe/workload/workload.proto",
        ])
        .status()
        .unwrap();

    assert!(status.success());

    tonic_build::configure()
        .compile_well_known_types(true)
        .type_attribute(
            "google.protobuf.Struct",
            "#[derive(::serde::Deserialize)] #[serde(transparent)]",
        )
        .type_attribute(
            "google.protobuf.Value",
            "#[derive(::serde::Deserialize)] #[serde(transparent)]",
        )
        .type_attribute(
            "google.protobuf.ListValue",
            "#[derive(::serde::Deserialize)] #[serde(transparent)]",
        )
        .type_attribute(
            "google.protobuf.NullValue",
            "#[derive(::serde_repr::Deserialize_repr)]",
        )
        .type_attribute(
            "google.protobuf.Value.kind",
            "#[derive(::serde::Deserialize)] #[serde(untagged)]",
        )
        .field_attribute("google.protobuf.Value.kind", "#[serde(flatten)]")
        .compile(&[proto], &[out_dir])
        .unwrap();
}
