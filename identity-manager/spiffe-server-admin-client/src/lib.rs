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

mod spiffe_connector;
mod spiffe_http_client;

pub use spiffe_connector::SpiffeConnector;
pub use spiffe_http_client::SpiffeHttpClient;
