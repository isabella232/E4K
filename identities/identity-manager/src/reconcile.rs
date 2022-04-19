// Copyright (c) Microsoft. All rights reserved.

use std::{collections::HashMap, path::PathBuf};

use core_objects::RegistrationEntry;
use tokio::{fs::File, io::AsyncReadExt};

use crate::config::Config;
use spiffe_server_admin_client::{SpiffeConnector, SpiffeHttpClient};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync + 'static>>;

pub struct Reconciler {
    config_path: PathBuf,
}

impl Reconciler {
    #[must_use]
    pub fn new(config_path: PathBuf) -> Self {
        Self { config_path }
    }

    pub async fn reconcile(&self) -> Result<()> {
        let config = self.read_config().await?;
        println!("Reading socket at {}", &config.server_socket_path);
        let connector = SpiffeHttpClient::new(&config.server_socket_path)?;

        self.apply_config(config, &connector).await?;
        Ok(())
    }

    async fn apply_config(&self, config: Config, connector: &dyn SpiffeConnector) -> Result<()> {
        let existing_identities = connector.get_identities().await?;

        let mut existing_identities: HashMap<String, RegistrationEntry> = existing_identities
            .into_iter()
            .map(|i| (i.id.clone(), i))
            .collect();

        let mut entries_to_add = Vec::new();
        let mut entries_to_delete = Vec::new();
        for entry in config.entries {
            let mut config_entry = RegistrationEntry {
                id: entry.spiffe_id,
                spiffe_id_path: entry.spiffe_id_path,
                other_identities: entry.other_identities.iter().map(|i| i.0.clone()).collect(),
                attestation_config: entry.attestation_config,
                admin: true,
                expires_at: u64::MAX,
                dns_names: vec!["mydns".to_string()],
                revision_number: 1,
                store_svid: true,
            };

            if let Some(actual_entry) = existing_identities.remove(&config_entry.id) {
                if should_modify_entry(&config_entry, &actual_entry) {
                    config_entry.revision_number = actual_entry.revision_number + 1;

                    entries_to_delete.push(config_entry.id.clone());
                    entries_to_add.push(config_entry);
                }
            } else {
                entries_to_add.push(config_entry);
            }
        }

        // The only entries left weren't in the config
        for (id, _) in existing_identities {
            entries_to_delete.push(id);
        }

        if !entries_to_delete.is_empty() {
            println!("Deleting {} entries.", entries_to_delete.len());
            connector.delete_identities(entries_to_delete).await?;
        }
        if !entries_to_add.is_empty() {
            println!("Adding {} entries.", entries_to_add.len());
            connector.create_identities(entries_to_add).await?;
        }

        Ok(())
    }

    async fn read_config(&self) -> Result<Config> {
        let mut raw_config = File::open(&self.config_path).await?;
        let mut buf = Vec::new();
        raw_config.read_to_end(&mut buf).await?;

        let config: Config = toml::from_slice(&buf)?;
        Ok(config)
    }
}

fn should_modify_entry(config_entry: &RegistrationEntry, actual_entry: &RegistrationEntry) -> bool {
    actual_entry.id != config_entry.id
        || actual_entry.spiffe_id_path != config_entry.spiffe_id_path
        || actual_entry.other_identities != config_entry.other_identities
        || actual_entry.attestation_config != config_entry.attestation_config
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::config::Entry;
    use core_objects::{AttestationConfig, EntryNodeAttestation, NodeAttestationPlugin};
    use spiffe_server_admin_client::SpiffeFakeConnector;

    #[tokio::test]
    async fn test_add_entry() {
        let fake_connector = SpiffeFakeConnector::default();
        let reconciler = Reconciler::new("".into());

        let entry_to_add = Entry {
            spiffe_id: "1".to_owned(),
            spiffe_id_path: "test".to_owned(),
            attestation_config: AttestationConfig::Node(EntryNodeAttestation {
                value: Vec::new(),
                plugin: NodeAttestationPlugin::Psat,
            }),
            other_identities: Vec::new(),
            parent_id: None,
        };
        let config = Config {
            entries: vec![entry_to_add],
            ..Default::default()
        };

        // ====================================================================================================================

        let current_entries = fake_connector.get_identities().await.expect("get entries");
        assert_eq!(current_entries.len(), 0);

        reconciler
            .apply_config(config, &fake_connector)
            .await
            .expect("applying config");

        let current_entries = fake_connector.get_identities().await.expect("get entries");
        assert_eq!(current_entries.len(), 1);
        assert_eq!(current_entries[0].id, "1");
    }

    #[tokio::test]
    async fn test_remove_entry() {
        let existing_entry = RegistrationEntry {
            id: "2".to_string(),
            spiffe_id_path: "test2".to_string(),
            attestation_config: AttestationConfig::Node(EntryNodeAttestation {
                value: Vec::new(),
                plugin: NodeAttestationPlugin::Psat,
            }),
            other_identities: Default::default(),
            admin: Default::default(),
            expires_at: Default::default(),
            dns_names: Default::default(),
            revision_number: Default::default(),
            store_svid: Default::default(),
        };

        let fake_connector = SpiffeFakeConnector {
            current_identities: std::sync::Mutex::new(vec![existing_entry]),
            ..Default::default()
        };
        let reconciler = Reconciler::new("".into());

        // ====================================================================================================================

        let current_entries = fake_connector.get_identities().await.expect("get entries");
        assert_eq!(current_entries.len(), 1);

        reconciler
            .apply_config(Config::default(), &fake_connector)
            .await
            .expect("applying config");

        let current_entries = fake_connector.get_identities().await.expect("get entries");
        assert_eq!(current_entries.len(), 0);
    }

    #[tokio::test]
    async fn test_modify_entry() {
        let existing_entry = RegistrationEntry {
            id: "3".to_string(),
            spiffe_id_path: "test3".to_string(),
            attestation_config: AttestationConfig::Node(EntryNodeAttestation {
                value: Vec::new(),
                plugin: NodeAttestationPlugin::Psat,
            }),
            other_identities: Default::default(),
            admin: Default::default(),
            expires_at: Default::default(),
            dns_names: Default::default(),
            revision_number: 5,
            store_svid: Default::default(),
        };

        let fake_connector = SpiffeFakeConnector {
            current_identities: std::sync::Mutex::new(vec![existing_entry]),
            ..Default::default()
        };
        let reconciler = Reconciler::new("".into());

        let new_entry = Entry {
            spiffe_id: "3".to_owned(),
            spiffe_id_path: "test3".to_owned(),
            attestation_config: AttestationConfig::Node(EntryNodeAttestation {
                value: Vec::new(),
                // Note that the entry above has Psat. We are setting the value to Sat
                plugin: NodeAttestationPlugin::Sat,
            }),
            other_identities: Vec::new(),
            parent_id: None,
        };
        let config = Config {
            entries: vec![new_entry],
            ..Default::default()
        };

        // ==================================================================================================================

        let current_entries = fake_connector.get_identities().await.expect("get entries");
        assert_eq!(current_entries.len(), 1);
        assert_eq!(current_entries[0].id, "3");
        assert_eq!(current_entries[0].spiffe_id_path, "test3");
        assert_eq!(current_entries[0].revision_number, 5);
        assert_eq!(
            current_entries[0].attestation_config,
            AttestationConfig::Node(EntryNodeAttestation {
                value: Vec::new(),
                plugin: NodeAttestationPlugin::Psat, // Currently PSAT
            })
        );

        reconciler
            .apply_config(config, &fake_connector)
            .await
            .expect("applying config");

        let current_entries = fake_connector.get_identities().await.expect("get entries");
        assert_eq!(current_entries.len(), 1);
        assert_eq!(current_entries[0].id, "3");
        assert_eq!(current_entries[0].spiffe_id_path, "test3");
        assert_eq!(current_entries[0].revision_number, 6);
        assert_eq!(
            current_entries[0].attestation_config,
            AttestationConfig::Node(EntryNodeAttestation {
                value: Vec::new(),
                plugin: NodeAttestationPlugin::Sat, // After config update is now SAT
            })
        );
    }
}
