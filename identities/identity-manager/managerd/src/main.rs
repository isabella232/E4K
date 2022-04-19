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

use tokio::time::{sleep, Duration};

use identity_manager::Reconciler;

const CONFIG_DEFAULT_PATH: &str = "/mnt/config/Config.toml";
const RECONCILE_RETRY_SECS: u64 = 10;
const RECONCILE_FREQUENCY_MINS: u64 = 10;

#[tokio::main]
async fn main() {
    let path = if let Ok(path) = std::env::var("CONFIG_PATH") {
        path
    } else {
        CONFIG_DEFAULT_PATH.to_string()
    };

    println!("Reading config from {}", path);
    let reconciler = Reconciler::new(path.into());

    loop {
        loop {
            println!("Trying reconcile");
            if let Err(e) = reconciler.reconcile().await {
                println!(
                    "Error reconciling identities: {:#?}\nretrying in 10 seconds...",
                    e
                );
                sleep(Duration::from_secs(RECONCILE_RETRY_SECS)).await;
            } else {
                break;
            }
        }
        println!("Reconcile Succeded, sleeping for 10 minutes");
        sleep(Duration::from_secs(RECONCILE_FREQUENCY_MINS * 60)).await;
    }
}
