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

mod error;

use std::{collections::HashMap, sync::Arc};

use catalog::{Entries, NodeSelectorType};
use core_objects::{
    AttestationConfig, NodeSelector, RegistrationEntry, WorkloadSelector, WorkloadSelectorType,
    SPIFFEID,
};
use error::Error;

const PAGE_SIZE: usize = 100;

pub struct IdentityMatcher {
    catalog: Arc<dyn Entries + Sync + Send>,
}

impl IdentityMatcher {
    #[must_use]
    pub fn new(catalog: Arc<dyn Entries + Sync + Send>) -> Self {
        Self { catalog }
    }

    pub async fn get_identities_on_behalf(
        &self,
        workload_selectors: &[WorkloadSelector],
        parent_selectors: &HashMap<NodeSelectorType, NodeSelector>,
    ) -> Result<Vec<SPIFFEID>, Error> {
        let workload_selectors = map_selectors(workload_selectors);
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
                    .match_entry(&workload_selectors, &entry, parent_selectors)
                    .await?;

                // If we have a match add the spiffe ID to the list
                if result {
                    identities.push(entry.spiffe_id);
                }
            }

            if token.is_none() {
                return Ok(identities);
            }
        }
    }

    async fn match_entry(
        &self,
        workload_selectors: &HashMap<WorkloadSelectorType, &WorkloadSelector>,
        entry: &RegistrationEntry,
        parent_selectors: &HashMap<NodeSelectorType, NodeSelector>,
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
                    match_workload_selectors(&workload_attestation.value, workload_selectors)
                        & match_node_selectors(&node_attestation.value, parent_selectors),
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

fn match_node_selectors(
    entry_selectors: &[NodeSelector],
    selectors: &HashMap<NodeSelectorType, NodeSelector>,
) -> bool {
    for expected_selector in entry_selectors {
        let selector_type = NodeSelectorType::from(expected_selector);
        let current_selector = if let Some(selector) = selectors.get(&selector_type) {
            selector
        } else {
            return false;
        };

        if current_selector != expected_selector {
            return false;
        }
    }

    true
}

// Considered doing a common match selectors for both workload and node like this:
// fn match_selectors<A, B, F>(entry_selectors: &Vec<B>, selectors: &HashMap<A, &B>, f: F) -> bool
// Put the hash map is slightly different. Changing the hash map to have a common function would be more expensive.
fn match_workload_selectors(
    entry_selectors: &[WorkloadSelector],
    selectors: &HashMap<WorkloadSelectorType, &WorkloadSelector>,
) -> bool {
    for expected_selector in entry_selectors {
        let selector_type = WorkloadSelectorType::from(expected_selector);
        let current_selector = if let Some(selector) = selectors.get(&selector_type) {
            selector
        } else {
            return false;
        };

        if *current_selector != expected_selector {
            return false;
        }
    }

    true
}

fn map_selectors(
    selectors: &'_ [WorkloadSelector],
) -> HashMap<WorkloadSelectorType, &'_ WorkloadSelector> {
    let mut selectors_map = HashMap::new();
    for selector in selectors {
        let selector_type = WorkloadSelectorType::from(selector);
        selectors_map.insert(selector_type, selector);
    }

    selectors_map
}

#[cfg(test)]
mod tests {
    use catalog::inmemory;
    use core_objects::{
        EntryNodeAttestation, EntryWorkloadAttestation, NodeAttestationPlugin, NodeSelector,
        WorkloadAttestationPlugin::K8s, CONFIG_DEFAULT_PATH,
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
        let config = Config::load_config(CONFIG_DEFAULT_PATH).unwrap();
        let catalog = Arc::new(inmemory::Catalog::new());

        let parent_spiffe_id = SPIFFEID {
            trust_domain: config.trust_domain.clone(),
            path: PARENT_NAME.to_string(),
        };

        // Add parent
        let parent = RegistrationEntry {
            id: PARENT_NAME.to_string(),
            other_identities: Vec::new(),
            spiffe_id: parent_spiffe_id.clone(),
            attestation_config: AttestationConfig::Node(EntryNodeAttestation {
                value: vec![
                    NodeSelector::Cluster(CLUSTER_NAME.to_string()),
                    NodeSelector::AgentNameSpace("selector2".to_string()),
                ],
                plugin: NodeAttestationPlugin::Sat,
            }),
            admin: false,
            ttl: 0,
            expires_at: 0,
            dns_names: Vec::new(),
            revision_number: 0,
            store_svid: false,
        };
        catalog.batch_create(vec![parent.clone()]).await.unwrap();

        // Add pod 1
        let mut entry1 = parent.clone();
        entry1.id = POD_NAME1.to_string();
        entry1.spiffe_id.path = POD_NAME1.to_string();
        entry1.attestation_config = AttestationConfig::Workload(EntryWorkloadAttestation {
            parent_id: PARENT_NAME.to_string(),
            value: vec![WorkloadSelector::PodName(POD_NAME1.to_string())],
            plugin: K8s,
        });
        catalog.batch_create(vec![entry1.clone()]).await.unwrap();

        // Add pod 2
        let mut entry2 = parent.clone();
        entry2.id = POD_NAME2.to_string();
        entry2.spiffe_id.path = POD_NAME2.to_string();
        entry2.attestation_config = AttestationConfig::Workload(EntryWorkloadAttestation {
            parent_id: PARENT_NAME.to_string(),
            value: vec![WorkloadSelector::PodName(POD_NAME2.to_string())],
            plugin: K8s,
        });
        catalog.batch_create(vec![entry2.clone()]).await.unwrap();

        // Add group
        let mut group = parent.clone();
        group.id = GROUP_NAME.to_string();
        group.spiffe_id.path = GROUP_NAME.to_string();
        group.attestation_config = AttestationConfig::Workload(EntryWorkloadAttestation {
            parent_id: PARENT_NAME.to_string(),
            value: vec![WorkloadSelector::ServiceAccount(GROUP_NAME.to_string())],
            plugin: K8s,
        });
        catalog.batch_create(vec![group.clone()]).await.unwrap();

        (IdentityMatcher::new(catalog), parent, entry1, entry2, group)
    }

    fn check_if_spiffe_id_in_response(response: Vec<SPIFFEID>, spiffe_id: &SPIFFEID) -> bool {
        let mut result: bool = false;

        for resp_spiffe_id in response {
            if resp_spiffe_id.to_string() == spiffe_id.to_string() {
                result = true;
            }
        }

        result
    }

    fn get_workload_selectors(entry: &RegistrationEntry) -> Vec<WorkloadSelector> {
        if let AttestationConfig::Workload(workload_attestation) = &entry.attestation_config {
            workload_attestation.value.clone()
        } else {
            panic!("Error, entry should be workload attestation");
        }
    }

    fn get_node_selectors(entry: &RegistrationEntry) -> HashMap<NodeSelectorType, NodeSelector> {
        let selectors_vec =
            if let AttestationConfig::Node(node_attestation) = &entry.attestation_config {
                node_attestation.value.clone()
            } else {
                panic!("Error, entry should be node attestation");
            };

        let mut selectors = HashMap::new();

        for selector in selectors_vec {
            selectors.insert(NodeSelectorType::from(&selector), selector);
        }

        selectors
    }

    #[tokio::test]
    async fn get_identities_on_behalf_happy_path_test() {
        let (identity_matcher, parent, entry1, _entry2, group) = init_test().await;

        let entry1_selectors = get_workload_selectors(&entry1);
        let parent_selectors = get_node_selectors(&parent);

        let mut workload_selectors = entry1_selectors.clone();
        // Push some other dummy selectors. Additional selectors should be ignored.
        // The important part is that all the entry selectors need to be mapped.
        workload_selectors.push(WorkloadSelector::ServiceAccount("dummy".to_string()));

        let mut parent_selectors = parent_selectors.clone();
        parent_selectors.insert(
            NodeSelectorType::AgentServiceAccount,
            NodeSelector::AgentServiceAccount("dummy".to_string()),
        );
        let identities = identity_matcher
            .get_identities_on_behalf(&workload_selectors, &parent_selectors)
            .await
            .unwrap();

        assert_eq!(1, identities.len());
        assert!(check_if_spiffe_id_in_response(
            identities,
            &entry1.spiffe_id.clone()
        ));

        // Now the workload should match both pod1 and group identity
        let mut group_selectors = get_workload_selectors(&group);
        workload_selectors.append(&mut group_selectors);
        let identities = identity_matcher
            .get_identities_on_behalf(&workload_selectors, &parent_selectors)
            .await
            .unwrap();
        assert_eq!(2, identities.len());
        assert!(check_if_spiffe_id_in_response(
            identities.clone(),
            &entry1.spiffe_id
        ));
        assert!(check_if_spiffe_id_in_response(identities, &group.spiffe_id));
    }

    #[tokio::test]
    async fn get_identities_on_behalf_error_match_test() {
        let (identity_matcher, parent, entry1, _entry2, _group) = init_test().await;

        let entry1_selectors = get_workload_selectors(&entry1);
        let parent_selectors = get_node_selectors(&parent);

        let mut workload_selectors = entry1_selectors.clone();
        // Push some other dummy selectors. Additional selectors should be ignored.
        // The important part is that all the entry selectors need to be mapped.
        workload_selectors.push(WorkloadSelector::ServiceAccount("dummy".to_string()));

        let mut parent_selectors = parent_selectors.clone();
        parent_selectors.insert(
            NodeSelectorType::AgentServiceAccount,
            NodeSelector::AgentServiceAccount("dummy".to_string()),
        );

        // Delete parent entry to create an error.
        identity_matcher
            .catalog
            .batch_delete(&[parent.id.clone()])
            .await
            .unwrap();
        let error = identity_matcher
            .get_identities_on_behalf(&workload_selectors, &parent_selectors)
            .await
            .unwrap_err();
        assert_matches!(error, Error::CatalogGetEntries(_));
    }

    #[tokio::test]
    async fn match_entry_happy_path() {
        let (identity_matcher, parent, entry1, _entry2, _group) = init_test().await;

        let entry1_selectors = &get_workload_selectors(&entry1);
        let workload_selectors = map_selectors(entry1_selectors);
        let parent_selectors = get_node_selectors(&parent);

        let result = identity_matcher
            .match_entry(&workload_selectors, &entry1, &parent_selectors)
            .await
            .unwrap();
        assert!(result);
    }

    #[tokio::test]
    async fn match_entry_cannot_get_entry_test() {
        let (identity_matcher, parent, entry1, _entry2, _group) = init_test().await;

        // Test the error case. What happens we have a workload entry that refers to a non-existing parent entry.
        let entry1_selectors = &get_workload_selectors(&entry1);
        let workload_selectors = map_selectors(entry1_selectors);
        let parent_selectors = get_node_selectors(&parent);

        // Delete parent entry to create an error.
        identity_matcher
            .catalog
            .batch_delete(&[parent.id.clone()])
            .await
            .unwrap();

        let error = identity_matcher
            .match_entry(&workload_selectors, &entry1, &parent_selectors)
            .await
            .unwrap_err();
        assert_matches!(error, Error::CatalogGetEntries(_));
    }

    #[tokio::test]
    async fn match_entry_cannot_match_parent_test() {
        let (identity_matcher, parent, entry1, _entry2, _group) = init_test().await;

        // We try to put a parent as the entry. It should never match.
        let entry1_selectors = &get_workload_selectors(&entry1);
        let workload_selectors = map_selectors(entry1_selectors);
        let parent_selectors = get_node_selectors(&parent);

        let result = identity_matcher
            .match_entry(&workload_selectors, &parent, &parent_selectors)
            .await
            .unwrap();
        assert!(!result);
    }

    #[tokio::test]
    async fn match_entry_bad_worload_selector_test() {
        let (identity_matcher, parent, _entry1, _entry2, _group) = init_test().await;

        // In this test we put a one entry with workload that do not match it.
        let mut workload_selectors = HashMap::new();
        let parent_selectors = get_node_selectors(&parent);

        // entry1 specifies a pod name, so a wrong selector type will not match
        let selector = &WorkloadSelector::PodUID("dummy".to_string());
        workload_selectors.insert(WorkloadSelectorType::PodUID, selector);
        let result = identity_matcher
            .match_entry(&workload_selectors, &parent, &parent_selectors)
            .await
            .unwrap();
        assert!(!result);

        // This time try with the correct selector but wong value.
        let selector = &WorkloadSelector::PodName("dummy".to_string());
        workload_selectors.insert(WorkloadSelectorType::PodName, selector);
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
        let entry1_selectors = &get_workload_selectors(&entry1);
        let workload_selectors = map_selectors(entry1_selectors);
        let mut parent_selectors = HashMap::new();
        parent_selectors.insert(
            NodeSelectorType::AgentPodName,
            NodeSelector::AgentPodName("dummy".to_string()),
        );

        let result = identity_matcher
            .match_entry(&workload_selectors, &entry1, &parent_selectors)
            .await
            .unwrap();
        assert!(!result);
    }
}
