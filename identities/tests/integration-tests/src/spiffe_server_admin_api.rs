// Copyright (c) Microsoft. All rights reserved.

#[cfg(test)]
mod tests {
    use core_objects::{
        AttestationConfig, EntryNodeAttestation, NodeAttestationPlugin, RegistrationEntry,
    };
    use spiffe_server_admin_client::{SpiffeConnector, SpiffeHttpClient};
    use std::sync::Arc;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn connect_to_socket() {
        let tmp = tempfile::tempdir().unwrap();
        let server = start_test_server(&tmp).await;
        let client = SpiffeHttpClient::new(&server.socket).expect("Could not make Spiffe Client");
        client.get_identities().await.expect("Can get identities");
    }

    #[tokio::test]
    async fn basic_crud() {
        let tmp = tempfile::tempdir().unwrap();
        let server = start_test_server(&tmp).await;
        let client = SpiffeHttpClient::new(&server.socket).expect("Could not make Spiffe Client");

        // ======= get identities ====================================================================
        let current_identites = client.get_identities().await.expect("Can get identities");
        assert_eq!(0, current_identites.len());

        // ======= create identities ================================================================
        let modules_to_create: Vec<String> = (0..10).map(|i| format!("Module {}", i)).collect();
        let identities_to_create = modules_to_create
            .iter()
            .map(|id| RegistrationEntry {
                id: id.clone(),
                other_identities: Vec::new(),
                spiffe_id_path: "path".to_string(),
                attestation_config: AttestationConfig::Node(EntryNodeAttestation {
                    value: Vec::new(),
                    plugin: NodeAttestationPlugin::Psat,
                }),
                admin: false,
                expires_at: 1028,
                dns_names: Vec::new(),
                revision_number: 0,
                store_svid: false,
            })
            .collect();

        client
            .create_identities(identities_to_create)
            .await
            .expect("Can create identities");

        let current_identites = client.get_identities().await.expect("Can get identities");
        assert_eq!(10, current_identites.len());

        let mut current_ids: Vec<String> = current_identites.iter().map(|e| e.id.clone()).collect();
        current_ids.sort();
        assert_eq!(modules_to_create, current_ids);

        // ======= delete identities ======================================================================
        let identities_to_delete: Vec<String> = (0..5).map(|i| format!("Module {}", i)).collect();
        client
            .delete_identities(identities_to_delete)
            .await
            .expect("Can delete identities");

        let current_identites = client.get_identities().await.expect("Can get identities");
        assert_eq!(5, current_identites.len());

        let mut current_ids: Vec<String> = current_identites.iter().map(|e| e.id.clone()).collect();
        current_ids.sort();
        let expected_ids: Vec<String> = (5..10).map(|i| format!("Module {}", i)).collect();
        assert_eq!(expected_ids, current_ids);
    }

    #[tokio::test]
    async fn paginated_get() {
        let tmp = tempfile::tempdir().unwrap();
        let server = start_test_server(&tmp).await;
        let client = SpiffeHttpClient::new(&server.socket).expect("Could not make Spiffe Client");

        let current_identites = client.get_identities().await.expect("Can get identities");
        assert_eq!(0, current_identites.len());

        // create lots of identities
        let modules_to_create: Vec<String> = (0..1000).map(|i| format!("Module {}", i)).collect();
        let identities_to_create = modules_to_create
            .iter()
            .map(|id| RegistrationEntry {
                id: id.clone(),
                other_identities: Vec::new(),
                spiffe_id_path: "path".to_string(),
                attestation_config: AttestationConfig::Node(EntryNodeAttestation {
                    value: Vec::new(),
                    plugin: NodeAttestationPlugin::Psat,
                }),
                admin: false,
                expires_at: 1028,
                dns_names: Vec::new(),
                revision_number: 0,
                store_svid: false,
            })
            .collect();
        client
            .create_identities(identities_to_create)
            .await
            .expect("Can create identities");

        // make sure we get all 1000 back
        let current_identites = client.get_identities().await.expect("Can get identities");
        assert_eq!(1000, current_identites.len());

        let mut current_ids: Vec<String> = current_identites.iter().map(|e| e.id.clone()).collect();
        current_ids.sort();
        let mut expected_ids = modules_to_create;
        expected_ids.sort();
        assert_eq!(expected_ids, current_ids);
    }

    struct TestServer {
        socket: String,
    }

    async fn start_test_server(dir: &tempfile::TempDir) -> TestServer {
        let socket = dir
            .path()
            .join("api.sock")
            .as_os_str()
            .to_string_lossy()
            .to_string();

        let mut config = server_config::Config::load_config(
            "../../iot-edge-spiffe-server/config/tests/Config.toml",
        )
        .unwrap();

        tokio::spawn({
            let socket = socket.clone();

            async move {
                config.socket_path = socket;

                let catalog = Arc::new(catalog::inmemory::Catalog::new());

                admin_api::start_admin_api(&config, catalog).await.unwrap();
            }
        });
        sleep(Duration::from_millis(10)).await;

        TestServer { socket }
    }
}
