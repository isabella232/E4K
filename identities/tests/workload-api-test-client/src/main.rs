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
use std::{thread, time::Duration};
use tokio::net::UnixStream;
use tonic::transport::{Endpoint, Uri};
use tower::service_fn;
use workload_api::generated::{
    spiffe_workload_api_client::SpiffeWorkloadApiClient, JwtBundlesRequest, Jwtsvid,
    JwtsvidRequest, ValidateJwtsvidRequest,
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

    // Fetch trust bundle test
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

    // Fetch SVID test
    let request = JwtsvidRequest {
        audience: vec!["spiffe://iotedge/mqttbroker".to_string()],
        spiffe_id: String::new(),
    };
    let response = client.fetch_jwtsvid(request).await.unwrap();
    let svids = response.into_inner().svids;
    info!("Got svids {:?}", svids);

    // Validate SVID test
    let request = ValidateJwtsvidRequest {
        audience: "spiffe://iotedge/mqttbroker".to_string(),
        svid: svids[0].svid.clone(),
    };
    let response = client.validate_jwtsvid(request).await.unwrap();
    let claims = response.into_inner();
    info!("Got claims {:?}", claims);

    // MQTT test
    start_mqtt_test(&svids[0]);
}

fn start_mqtt_test(svid: &Jwtsvid) {
    let mut count = 0;

    loop {
        let opts = paho_mqtt::CreateOptionsBuilder::new()
            .server_uri("tcp://mqttbroker:1883")
            .client_id(&svid.spiffe_id)
            .finalize();
        let mut cli = paho_mqtt::Client::new(opts).unwrap();
        let mut copts_builder = paho_mqtt::ConnectOptionsBuilder::new();

        // Use 5sec timeouts for sync calls.
        cli.set_timeout(Duration::from_secs(5));

        let rx = cli.start_consuming();

        copts_builder.user_name(&svid.spiffe_id);
        copts_builder.password(&svid.svid);

        let copts = copts_builder.finalize();

        // Connect and wait for it to complete or fail
        cli.connect(Some(copts)).unwrap();

        cli.subscribe("test", 0).unwrap();

        // Create a message and publish it
        let msg = paho_mqtt::MessageBuilder::new()
            .topic("test")
            .payload(format!("Message #{}", count))
            .qos(1)
            .finalize();

        count += 1;

        if let Err(e) = cli.publish(msg) {
            println!("Error sending message: {:?}", e);
        }
        let recv = rx.recv();

        if let Ok(Some(message)) = recv {
            println!(
                "Message received: {}",
                String::from_utf8_lossy(message.payload())
            );
        }

        // Disconnect from the broker
        cli.disconnect(None).unwrap();

        thread::sleep(Duration::from_millis(2000));
    }
}
