// Copyright (c) Microsoft. All rights reserved.
#![deny(rust_2018_idioms)]
#![warn(clippy::all, clippy::pedantic)]
#![allow(
    clippy::missing_safety_doc,
    clippy::default_trait_access,
    clippy::let_unit_value,
    clippy::missing_errors_doc,
    clippy::similar_names,
    clippy::too_many_lines
)]

pub mod error;

use std::{collections::BTreeSet, sync::Arc};

use catalog::Catalog;
use core_objects::{AttestationConfig, RegistrationEntry};
use error::Error;

const PAGE_SIZE: usize = 100;

pub struct IdentityMatcher {
    catalog: Arc<dyn Catalog>,
}

impl IdentityMatcher {
    #[must_use]
    pub fn new(catalog: Arc<dyn Catalog>) -> Self {
        Self { catalog }
    }

    pub async fn get_entry_id_from_selectors(
        &self,
        workload_selectors: &BTreeSet<String>,
        parent_selectors: &BTreeSet<String>,
    ) -> Result<Vec<RegistrationEntry>, Error> {
        let mut identities = Vec::new();

        loop {
            let (entries, token) = self
                .catalog
                .list_all(None, PAGE_SIZE)
                .await
                .map_err(Error::CatalogGetEntries)?;

            // Go over all the entries. For each entry, we check if the workload that just came up is matching any of the entries we have.
            // For each matching entry, we will extract the SPIFFE identity and match it with the workload.
            for entry in entries {
                // Check if the workload selectors are matching with the entry.
                let result = self
                    .match_entry(workload_selectors, &entry, parent_selectors)
                    .await?;

                // If we have a match add the ID to the list
                if result {
                    identities.push(entry);
                }
            }

            if token.is_none() {
                return Ok(identities);
            }
        }
    }

    async fn match_entry(
        &self,
        workload_selectors: &BTreeSet<String>,
        entry: &RegistrationEntry,
        parent_selectors: &BTreeSet<String>,
    ) -> Result<bool, Error> {
        // Get the selectors for the entry and the parent entry. Those selectors will be checked againt the selectors of
        // the workload and the parent making the request on behalf of the workload.
        // To have a match, all the entry selectors need to be present in node/workload selector set.
        if let AttestationConfig::Workload(workload_attestation) = &entry.attestation_config {
            let parent_entry = self
                .catalog
                .get_entry(&workload_attestation.parent_id)
                .await
                .map_err(Error::CatalogGetEntries)?;

            if let AttestationConfig::Node(node_attestation) = &parent_entry.attestation_config {
                Ok(
                    match_selectors(&workload_attestation.value, workload_selectors)
                        & match_selectors(&node_attestation.value, parent_selectors),
                )
            } else {
                // This error is when a regular workload is parented to another workload.
                // This error should be filtered out when entries are created, not here.
                // We don't want to error the process for an invalid entry.
                log::error!("Entry {} was parented to another workload", entry.id);
                Ok(false)
            }
        } else {
            // Ignore parents, match only workloads.
            Ok(false)
        }
    }
}

fn match_selectors(entry_selectors: &[String], selectors: &BTreeSet<String>) -> bool {
    for expected_selector in entry_selectors {
        if !selectors.contains(expected_selector) {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use catalog::{inmemory, Entries};
    use core_objects::{
        build_selector_string, EntryNodeAttestation, EntryWorkloadAttestation,
        NodeAttestationPlugin, NodeSelectorType, WorkloadAttestationPlugin::K8s,
        WorkloadSelectorType, CONFIG_DEFAULT_PATH,
    };
    use matches::assert_matches;
    use server_config::Config;

    use super::*;

    const PARENT_NAME: &str = "parent";
    const POD_NAME1: &str = "pod1";
    const POD_NAME2: &str = "pod2";
    const GROUP_NAME: &str = "podGroup";
    const CLUSTER_NAME: &str = "myCluster";

    async fn init_test() -> (
        IdentityMatcher,
        RegistrationEntry,
        RegistrationEntry,
        RegistrationEntry,
        RegistrationEntry,
    ) {
        let _config = Config::load_config(CONFIG_DEFAULT_PATH).unwrap();
        let catalog = Arc::new(inmemory::Catalog::new());

        // Add parent
        let parent = RegistrationEntry {
            id: PARENT_NAME.to_string(),
            other_identities: Vec::new(),
            spiffe_id_path: PARENT_NAME.to_string(),
            attestation_config: AttestationConfig::Node(EntryNodeAttestation {
                value: vec![
                    build_selector_string(&NodeSelectorType::Cluster, CLUSTER_NAME),
                    build_selector_string(&NodeSelectorType::AgentNameSpace, "selector2"),
                ],
                plugin: NodeAttestationPlugin::Sat,
            }),
            admin: false,
            expires_at: 0,
            dns_names: Vec::new(),
            revision_number: 0,
            store_svid: false,
        };
        catalog.batch_create(vec![parent.clone()]).await.unwrap();

        // Add pod 1
        let mut entry1 = parent.clone();
        entry1.id = POD_NAME1.to_string();
        entry1.spiffe_id_path = POD_NAME1.to_string();
        entry1.attestation_config = AttestationConfig::Workload(EntryWorkloadAttestation {
            parent_id: PARENT_NAME.to_string(),
            value: vec![build_selector_string(
                &WorkloadSelectorType::PodName,
                POD_NAME1,
            )],
            plugin: K8s,
        });
        catalog.batch_create(vec![entry1.clone()]).await.unwrap();

        // Add pod 2
        let mut entry2 = parent.clone();
        entry2.id = POD_NAME2.to_string();
        entry2.spiffe_id_path = POD_NAME2.to_string();
        entry2.attestation_config = AttestationConfig::Workload(EntryWorkloadAttestation {
            parent_id: PARENT_NAME.to_string(),
            value: vec![build_selector_string(
                &WorkloadSelectorType::PodName,
                POD_NAME2,
            )],
            plugin: K8s,
        });
        catalog.batch_create(vec![entry2.clone()]).await.unwrap();

        // Add group
        let mut group = parent.clone();
        group.id = GROUP_NAME.to_string();
        group.spiffe_id_path = GROUP_NAME.to_string();
        group.attestation_config = AttestationConfig::Workload(EntryWorkloadAttestation {
            parent_id: PARENT_NAME.to_string(),
            value: vec![build_selector_string(
                &WorkloadSelectorType::PodName,
                GROUP_NAME,
            )],
            plugin: K8s,
        });
        catalog.batch_create(vec![group.clone()]).await.unwrap();

        (IdentityMatcher::new(catalog), parent, entry1, entry2, group)
    }

    fn check_if_entry_id_in_response(response: Vec<RegistrationEntry>, id: &str) -> bool {
        let mut result: bool = false;

        for entries in response {
            if entries.id == id {
                result = true;
            }
        }

        result
    }

    fn get_workload_selectors(entry: &RegistrationEntry) -> BTreeSet<String> {
        let selectors_vec =
            if let AttestationConfig::Workload(workload_attestation) = &entry.attestation_config {
                workload_attestation.value.clone()
            } else {
                panic!("Error, entry should be workload attestation");
            };

        let mut selectors = BTreeSet::new();

        for selector in selectors_vec {
            selectors.insert(selector);
        }

        selectors
    }

    fn get_node_selectors(entry: &RegistrationEntry) -> BTreeSet<String> {
        let selectors_vec =
            if let AttestationConfig::Node(node_attestation) = &entry.attestation_config {
                node_attestation.value.clone()
            } else {
                panic!("Error, entry should be node attestation");
            };

        let mut selectors = BTreeSet::new();

        for selector in selectors_vec {
            selectors.insert(selector);
        }

        selectors
    }

    #[tokio::test]
    async fn get_entry_id_from_selectors_happy_path_test() {
        let (identity_matcher, parent, entry1, _entry2, group) = init_test().await;

        let entry1_selectors = get_workload_selectors(&entry1);
        let parent_selectors = get_node_selectors(&parent);

        let mut workload_selectors = entry1_selectors.clone();
        // Push some other dummy selectors. Additional selectors should be ignored.
        // The important part is that all the entry selectors need to be mapped.
        workload_selectors.insert(build_selector_string(
            &WorkloadSelectorType::ServiceAccount,
            "dummy",
        ));

        let mut parent_selectors = parent_selectors.clone();
        parent_selectors.insert(build_selector_string(
            &NodeSelectorType::AgentServiceAccount,
            "dummy",
        ));
        let entries = identity_matcher
            .get_entry_id_from_selectors(&workload_selectors, &parent_selectors)
            .await
            .unwrap();

        assert_eq!(1, entries.len());
        assert!(check_if_entry_id_in_response(entries, &entry1.id));

        // Now the workload should match both pod1 and group identity
        let mut group_selectors = get_workload_selectors(&group);
        workload_selectors.append(&mut group_selectors);
        let entries = identity_matcher
            .get_entry_id_from_selectors(&workload_selectors, &parent_selectors)
            .await
            .unwrap();
        assert_eq!(2, entries.len());
        assert!(check_if_entry_id_in_response(entries.clone(), &entry1.id));
        assert!(check_if_entry_id_in_response(entries, &group.id));
    }

    #[tokio::test]
    async fn get_entry_id_from_selectors_error_match_test() {
        let (identity_matcher, parent, entry1, _entry2, _group) = init_test().await;

        let entry1_selectors = get_workload_selectors(&entry1);
        let parent_selectors = get_node_selectors(&parent);

        let mut workload_selectors = entry1_selectors.clone();
        // Push some other dummy selectors. Additional selectors should be ignored.
        // The important part is that all the entry selectors need to be mapped.
        workload_selectors.insert(build_selector_string(
            &WorkloadSelectorType::ServiceAccount,
            "dummy",
        ));

        let mut parent_selectors = parent_selectors.clone();
        parent_selectors.insert(build_selector_string(
            &WorkloadSelectorType::ServiceAccount,
            "dummy",
        ));

        // Delete parent entry to create an error.
        identity_matcher
            .catalog
            .batch_delete(&[parent.id.clone()])
            .await
            .unwrap();
        let error = identity_matcher
            .get_entry_id_from_selectors(&workload_selectors, &parent_selectors)
            .await
            .unwrap_err();
        assert_matches!(error, Error::CatalogGetEntries(_));
    }

    #[tokio::test]
    async fn match_entry_happy_path() {
        let (identity_matcher, parent, entry1, _entry2, _group) = init_test().await;

        let workload_selectors = &get_workload_selectors(&entry1);
        let parent_selectors = get_node_selectors(&parent);

        let result = identity_matcher
            .match_entry(workload_selectors, &entry1, &parent_selectors)
            .await
            .unwrap();
        assert!(result);
    }

    #[tokio::test]
    async fn match_entry_cannot_get_entry_test() {
        let (identity_matcher, parent, entry1, _entry2, _group) = init_test().await;

        // Test the error case. What happens we have a workload entry that refers to a non-existing parent entry.
        let workload_selectors = &get_workload_selectors(&entry1);
        let parent_selectors = get_node_selectors(&parent);

        // Delete parent entry to create an error.
        identity_matcher
            .catalog
            .batch_delete(&[parent.id.clone()])
            .await
            .unwrap();

        let error = identity_matcher
            .match_entry(workload_selectors, &entry1, &parent_selectors)
            .await
            .unwrap_err();
        assert_matches!(error, Error::CatalogGetEntries(_));
    }

    #[tokio::test]
    async fn match_entry_cannot_match_parent_test() {
        let (identity_matcher, parent, entry1, _entry2, _group) = init_test().await;

        // We try to put a parent as the entry. It should never match.
        let workload_selectors = &get_workload_selectors(&entry1);
        let parent_selectors = get_node_selectors(&parent);

        let result = identity_matcher
            .match_entry(workload_selectors, &parent, &parent_selectors)
            .await
            .unwrap();
        assert!(!result);
    }

    #[tokio::test]
    async fn match_entry_bad_worload_selector_test() {
        let (identity_matcher, parent, _entry1, _entry2, _group) = init_test().await;

        // In this test we put a one entry with workload that do not match it.
        let mut workload_selectors = BTreeSet::new();
        let parent_selectors = get_node_selectors(&parent);

        // entry1 specifies a pod name, so a wrong selector type will not match
        workload_selectors.insert(build_selector_string(
            &WorkloadSelectorType::PodUID,
            "dummy",
        ));
        let result = identity_matcher
            .match_entry(&workload_selectors, &parent, &parent_selectors)
            .await
            .unwrap();
        assert!(!result);

        // This time try with the correct selector but wong value.
        workload_selectors.insert(build_selector_string(
            &WorkloadSelectorType::PodName,
            "dummy",
        ));
        let result = identity_matcher
            .match_entry(&workload_selectors, &parent, &parent_selectors)
            .await
            .unwrap();
        assert!(!result);
    }

    #[tokio::test]
    async fn match_entry_parent_do_not_match() {
        let (identity_matcher, _parent, entry1, _entry2, _group) = init_test().await;

        // Here the entry matches perfectly but parent do not match. Meaning the agent is forbidden
        // from requesting an svid for this entry.
        let workload_selectors = &get_workload_selectors(&entry1);
        let mut parent_selectors = BTreeSet::new();
        parent_selectors.insert(build_selector_string(
            &WorkloadSelectorType::ServiceAccount,
            "dummy",
        ));

        let result = identity_matcher
            .match_entry(workload_selectors, &entry1, &parent_selectors)
            .await
            .unwrap();
        assert!(!result);
    }
}
