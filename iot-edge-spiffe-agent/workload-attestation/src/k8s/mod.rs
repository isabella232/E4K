// Copyright (c) Microsoft. All rights reserved.

//! Workload attestation.
//!
//! The workload attestation will extract the platform primitive data that identifies a workload:
//! podname, poduid, node name, etc...
//! How it works:
//! The workloads reach the workload API through Unix Domain Socket (UDS). From the UDS we get the PID.
//! With the PID we get the cgroups. We regex the cgroups to get the pod uid and the container id.
//! Then we call kubernetes API to get the list of all the pod inside the node and we match the pod with the uid.
//! Once we find the pod we extract all the data (selectors)

pub mod error;

use agent_config::WorkloadAttestationConfigK8s;
use cgroups_rs::cgroup;
use core_objects::{build_selector_string, WorkloadSelectorType};
use k8s_openapi::{
    api::core::v1::{ContainerStatus, Pod},
    url::Url,
};
use log::{debug, info};
use regex::Regex;
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    time::Duration,
};
use tokio::time;

use crate::k8s::error::MissingField;
use crate::WorkloadAttributes;

use super::WorkloadAttestation as WorkloadAttestationTrait;

#[cfg(not(any(test, feature = "tests")))]
use kube::{Api, Client};
#[cfg(any(test, feature = "tests"))]
use mock_kube::{Api, Client};

use kube::{api::ListParams, core::ObjectList};

use error::Error;

const PID_CGROUP: &str = "pids";

// Regex taken from spire: https://github.com/spiffe/spire/blob/9fab47f081ca94517c1e0ac166f4afb2929f8ee8/pkg/agent/plugin/workloadattestor/k8s/k8s.go#L579
const REGEX_GET_UID: &str = "[[:punct:]]pod([[:xdigit:]]{8}[[:punct:]][[:xdigit:]]{4}[[:punct:]][[:xdigit:]]{4}[[:punct:]][[:xdigit:]]{4}[[:punct:]][[:xdigit:]]{12})[[:punct:]](?:[[:^punct:]]+[[:punct:]])*([[:^punct:]]+)$";

#[derive(Clone, Debug, Default)]
struct SelectorInfo {
    namespace: String,
    service_account_name: String,
    container_name: String,
    container_image: String,
    node_name: String,
    pod_labels: BTreeMap<String, String>,
    pod_owner: BTreeSet<String>,
    pod_owner_uid: BTreeSet<String>,
    pod_uid: String,
    pod_name: String,
    pod_image: BTreeSet<String>,
    pod_image_count: usize,
    pod_init_image: BTreeSet<String>,
    pod_init_image_count: usize,
}

#[derive(Clone, Debug, Default)]
struct ContainerIdentifiers {
    name: String,
    image: String,
}

pub struct WorkloadAttestation {
    node_name: String,
    client: Client,
    regex_get_uid: Regex,
    max_poll_attempt: usize,
    poll_retry_interval_ms: u64,
}

impl WorkloadAttestation {
    #[must_use]
    pub fn new(config: &WorkloadAttestationConfigK8s, node_name: String, client: Client) -> Self {
        let regex_get_uid = Regex::new(REGEX_GET_UID).unwrap();

        WorkloadAttestation {
            node_name,
            client,
            regex_get_uid,
            max_poll_attempt: config.max_poll_attempt,
            poll_retry_interval_ms: config.poll_retry_interval_ms,
        }
    }

    // For unit test, remove dependency to cgroup call.
    fn get_container_id_and_pod_uid_from_cgroup(
        &self,
        cgroups: &HashMap<String, String>,
    ) -> Result<(String, String), Error> {
        let path = cgroups
            .get(PID_CGROUP)
            .ok_or(Error::NoPIDcgroup)?
            .trim_end_matches(".scope");
        let captures = self
            .regex_get_uid
            .captures(path)
            .ok_or_else(|| Error::ExtractPodUIDandContainerID(path.to_string()))?;

        let pod_uid = canonicalize_pod_uid(&captures[1]);

        let container_id = captures[2].to_string();

        Ok((container_id, pod_uid))
    }

    async fn get_pod_list(&self) -> Result<ObjectList<Pod>, Error> {
        let pods: Api<Pod> = Api::default_namespaced(self.client.clone());
        let mut list_param = ListParams::default();
        let selector = format!("spec.nodeName={}", self.node_name);
        list_param.field_selector = Some(selector);

        pods.list(&list_param)
            .await
            .map_err(|error| Error::ListingPods {
                error,
                node_name: self.node_name.clone(),
            })
    }

    async fn get_pod(
        &self,
        container_id: &str,
        pod_uid: &str,
    ) -> Result<(Pod, ContainerIdentifiers), Error> {
        let mut attempt = 0;

        loop {
            let pod_list = self.get_pod_list().await?;

            for pod in pod_list {
                // If this is not the right pod, skip to the next one.
                if let Some(uid) = &pod.metadata.uid {
                    if uid != pod_uid {
                        continue;
                    }

                    // We found the pod, no need to continue return if good or exit the loop.
                    let container_identifiers = is_container_ready_in_pod(&pod, container_id);
                    if let Some(container_identifiers) = container_identifiers {
                        return Ok((pod, container_identifiers));
                    }
                    break;
                }
            }

            attempt += 1;
            if attempt >= self.max_poll_attempt {
                //Sleep until next attempt
                time::sleep(Duration::from_millis(self.poll_retry_interval_ms)).await;
                break;
            }
        }

        Err(Error::ContainerNotFoundInPod {
            container_id: container_id.to_string(),
            pod_uid: pod_uid.to_string(),
        })
    }

    async fn attest_workload_inner(
        &self,
        cgroups: HashMap<String, String>,
    ) -> Result<WorkloadAttributes, Error> {
        let (container_id, pod_uid) = self.get_container_id_and_pod_uid_from_cgroup(&cgroups)?;

        let (pod, container_identifier) = self.get_pod(&container_id, &pod_uid).await?;

        let selector_info = get_selector_info(pod, container_identifier)?;

        Ok(get_workload_attributes_from_select_info(&selector_info))
    }
}

#[async_trait::async_trait]
impl WorkloadAttestationTrait for WorkloadAttestation {
    async fn attest_workload(
        &self,
        pid: u32,
    ) -> Result<WorkloadAttributes, Box<dyn std::error::Error + Send>> {
        let cgroups =
            cgroup::get_cgroups_relative_paths_by_pid(pid).map_err(|err| Box::new(err) as _)?;
        // For unit test, we remove dependency to cgroup call.
        self.attest_workload_inner(cgroups)
            .await
            .map_err(|err| Box::new(err) as _)
    }
}

// canonicalizePodUID converts a Pod UID, as represented in a cgroup path, into
// a canonical form. Practically this means that we convert any punctuation to
// dashes, which is how the UID is represented within Kubernetes.
fn canonicalize_pod_uid(uid: &str) -> String {
    let uid = uid
        .chars()
        .map(|f| if f.is_ascii_punctuation() { '-' } else { f })
        .collect::<String>();

    uid
}

fn get_workload_attributes_from_select_info(selector_info: &SelectorInfo) -> WorkloadAttributes {
    let mut selectors = BTreeSet::new();
    selectors.insert(build_selector_string(
        &WorkloadSelectorType::Namespace,
        &selector_info.namespace,
    ));
    selectors.insert(build_selector_string(
        &WorkloadSelectorType::ServiceAccount,
        &selector_info.service_account_name,
    ));
    selectors.insert(build_selector_string(
        &WorkloadSelectorType::PodName,
        &selector_info.pod_name,
    ));
    selectors.insert(build_selector_string(
        &WorkloadSelectorType::PodUID,
        &selector_info.pod_uid,
    ));
    selectors.insert(build_selector_string(
        &WorkloadSelectorType::NodeName,
        &selector_info.node_name,
    ));
    selectors.insert(build_selector_string(
        &WorkloadSelectorType::ContainerName,
        &selector_info.container_name,
    ));
    selectors.insert(build_selector_string(
        &WorkloadSelectorType::ContainerImage,
        &selector_info.container_image,
    ));
    selectors.insert(build_selector_string(
        &WorkloadSelectorType::PodImageCount,
        &selector_info.pod_image_count,
    ));
    selectors.insert(build_selector_string(
        &WorkloadSelectorType::PodInitImageCount,
        &selector_info.pod_init_image_count,
    ));

    push_map_into_selectors(
        &mut selectors,
        &selector_info.pod_labels,
        &WorkloadSelectorType::PodLabels,
    );
    push_set_into_selectors(
        &mut selectors,
        &selector_info.pod_owner,
        &WorkloadSelectorType::PodOwners,
    );
    push_set_into_selectors(
        &mut selectors,
        &selector_info.pod_owner_uid,
        &WorkloadSelectorType::PodOwnerUIDs,
    );
    push_set_into_selectors(
        &mut selectors,
        &selector_info.pod_image,
        &WorkloadSelectorType::PodImages,
    );
    push_set_into_selectors(
        &mut selectors,
        &selector_info.pod_init_image,
        &WorkloadSelectorType::PodInitImages,
    );

    info!(
        "Workload {} was attested successfully",
        selector_info.pod_name
    );
    debug!("Found the following selectors for workload {:?}", selectors);

    WorkloadAttributes { selectors }
}

fn push_map_into_selectors<'a, A>(
    selectors: &mut BTreeSet<String>,
    map: &BTreeMap<String, String>,
    selector_type: &'a A,
) where
    &'a A: ToString + Clone,
{
    for (key, value) in map {
        let map = format!("{}:{}", key, value);
        let selector = build_selector_string(&selector_type, &map);
        selectors.insert(selector);
    }
}

fn push_set_into_selectors<'a, A>(
    selectors: &mut BTreeSet<String>,
    set: &BTreeSet<String>,
    selector_type: &'a A,
) where
    &'a A: ToString + Clone,
{
    for value in set {
        let selector = build_selector_string(&selector_type, &value);
        selectors.insert(selector);
    }
}

fn get_selector_info(
    pod: Pod,
    container_identifiers: ContainerIdentifiers,
) -> Result<SelectorInfo, Error> {
    let pod_spec = pod.spec.ok_or(Error::MissingField(MissingField::PodSpec))?;

    let owner_references = pod.metadata.owner_references.unwrap_or_default();

    let pod_status = pod
        .status
        .ok_or(Error::MissingField(MissingField::Status))?;

    let selector_info = SelectorInfo {
        pod_name: pod
            .metadata
            .name
            .ok_or(Error::MissingField(MissingField::PodName))?,
        pod_uid: pod
            .metadata
            .uid
            .ok_or(Error::MissingField(MissingField::PodUid))?,
        namespace: pod
            .metadata
            .namespace
            .ok_or(Error::MissingField(MissingField::Namespace))?,
        pod_labels: pod
            .metadata
            .labels
            .ok_or(Error::MissingField(MissingField::PodLabels))?,
        node_name: pod_spec
            .node_name
            .ok_or(Error::MissingField(MissingField::NodeName))?,
        service_account_name: pod_spec
            .service_account_name
            .ok_or(Error::MissingField(MissingField::ServiceAccountName))?,
        pod_owner: owner_references
            .iter()
            .map(|owner_ref| format!("{}:{}", owner_ref.kind, owner_ref.name))
            .collect(),
        pod_owner_uid: owner_references
            .iter()
            .map(|owner_ref| format!("{}:{}", owner_ref.kind, owner_ref.uid))
            .collect(),
        pod_image: pod_spec
            .containers
            .iter()
            .filter_map(|container| container.image.as_ref().cloned())
            .collect(),
        pod_init_image: pod_spec
            .init_containers
            .map_or_else(BTreeSet::new, |container| {
                container
                    .iter()
                    .filter_map(|container| container.image.as_ref().cloned())
                    .collect()
            }),
        pod_image_count: if let Some(statuses) = pod_status.container_statuses {
            statuses.len()
        } else {
            0
        },
        pod_init_image_count: if let Some(statuses) = pod_status.init_container_statuses {
            statuses.len()
        } else {
            0
        },
        container_name: container_identifiers.name,
        container_image: container_identifiers.image,
    };

    Ok(selector_info)
}

fn container_status_match_container_id(status: &ContainerStatus, container_id: &str) -> bool {
    let status_container_id_url = if let Some(status_container_id_url) = &status.container_id {
        status_container_id_url
    } else {
        return false;
    };

    let status_container_id_url =
        if let Ok(status_container_id_url) = Url::parse(status_container_id_url) {
            status_container_id_url
        } else {
            return false;
        };

    if let Some(host) = status_container_id_url.host() {
        host.to_string() == container_id
    } else {
        false
    }
}

fn is_container_ready_in_pod(pod: &Pod, container_id: &str) -> Option<ContainerIdentifiers> {
    let empty_vec = Vec::new();

    // Extract name
    let pod_name = if let Some(pod_name) = &pod.metadata.name {
        pod_name
    } else {
        info!("No pod name present in pod");
        return None;
    };

    // Extract status
    let status = if let Some(status) = &pod.status {
        status
    } else {
        info!("No pod status present in pod {}", pod_name);
        return None;
    };

    // Extract all containers status
    let container_statuses = if let Some(container_statuses) = &status.container_statuses {
        container_statuses
    } else {
        &empty_vec
    };

    // Extract all container init status
    let container_init_statuses =
        if let Some(container_init_statuses) = &status.init_container_statuses {
            container_init_statuses
        } else {
            &empty_vec
        };

    let container_identifiers = get_container_identitifiers(container_statuses, container_id);
    if container_identifiers.is_some() {
        return container_identifiers;
    }

    let container_identifiers = get_container_identitifiers(container_init_statuses, container_id);
    if container_identifiers.is_some() {
        return container_identifiers;
    }

    info!(
        "Was not able to find a status matching container id {} in pod {}",
        container_id, pod_name
    );
    None
}

fn get_container_identitifiers(
    container_statuses: &[ContainerStatus],
    container_id: &str,
) -> Option<ContainerIdentifiers> {
    for container_status in container_statuses {
        if container_status_match_container_id(container_status, container_id) {
            return Some(ContainerIdentifiers {
                name: container_status.name.clone(),
                image: container_status.image.clone(),
            });
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use kube::core::ListMeta;
    use matches::assert_matches;
    use mock_kube::{get_pods, CONTAINER_ID, INIT_CONTAINER_ID, POD_UID};

    use super::*;

    async fn init_selector_test() -> WorkloadAttestation {
        let workload_attestation_config = WorkloadAttestationConfigK8s {
            max_poll_attempt: 2,
            poll_retry_interval_ms: 0,
        };

        let client = Client::try_default().await.unwrap();
        WorkloadAttestation::new(&workload_attestation_config, "my_node".to_string(), client)
    }

    #[tokio::test]
    async fn attest_workload_inner_happy_path() {
        let mut workload_attestation = init_selector_test().await;

        let mut pod1 = get_pods();
        let pod2 = get_pods();
        pod1.metadata.uid = None;

        let pod_list = ObjectList {
            metadata: ListMeta::default(),
            items: vec![pod1, pod2],
        };

        let mut cgroups = HashMap::new();
        let path = format!(
            "/docker/{}/kubepods/besteffort/pod{}/{}",
            CONTAINER_ID, POD_UID, CONTAINER_ID
        );
        cgroups.insert("pids".to_string(), path);

        workload_attestation.client.queue_response(pod_list).await;
        let workload_selectors = workload_attestation
            .attest_workload_inner(cgroups)
            .await
            .unwrap()
            .selectors;

        let namespace = build_selector_string(&WorkloadSelectorType::Namespace, "namespace");
        assert!(workload_selectors.contains(&namespace));

        let service_account = build_selector_string(
            &WorkloadSelectorType::ServiceAccount,
            "iotedge-spiffe-agent",
        );
        assert!(workload_selectors.contains(&service_account));

        let service_account = build_selector_string(&WorkloadSelectorType::PodName, "pod_name");
        assert!(workload_selectors.contains(&service_account));

        let pod_uid = build_selector_string(
            &WorkloadSelectorType::PodUID,
            "75dbabec-9510-11ec-b909-0242ac120002",
        );
        assert!(workload_selectors.contains(&pod_uid));

        let node_name = build_selector_string(&WorkloadSelectorType::NodeName, "node_name");
        assert!(workload_selectors.contains(&node_name));

        let pod_label = build_selector_string(&WorkloadSelectorType::PodLabels, "pod-name:pod");
        assert!(workload_selectors.contains(&pod_label));

        let container_name =
            build_selector_string(&WorkloadSelectorType::ContainerName, "container_name");
        assert!(workload_selectors.contains(&container_name));

        let container_image = build_selector_string(&WorkloadSelectorType::ContainerImage, "image");
        assert!(workload_selectors.contains(&container_image));

        let pod_owners = build_selector_string(&WorkloadSelectorType::PodOwners, "kind:name");
        assert!(workload_selectors.contains(&pod_owners));

        let pod_owner_uid = build_selector_string(&WorkloadSelectorType::PodOwnerUIDs, "kind:uid");
        assert!(workload_selectors.contains(&pod_owner_uid));

        let pod_images = build_selector_string(&WorkloadSelectorType::PodImages, "ubuntu:latest");
        assert!(workload_selectors.contains(&pod_images));

        let image_count = build_selector_string(&WorkloadSelectorType::PodImageCount, "1");
        assert!(workload_selectors.contains(&image_count));

        let pod_init_images =
            build_selector_string(&WorkloadSelectorType::PodInitImages, "debian:latest");
        assert!(workload_selectors.contains(&pod_init_images));

        let init_image_count = build_selector_string(&WorkloadSelectorType::PodInitImageCount, "1");
        assert!(workload_selectors.contains(&init_image_count));
    }

    #[test]
    fn get_container_identitifiers_no_match() {
        let container_status = ContainerStatus {
            container_id: Some(format!("docker://{}", CONTAINER_ID)),
            image: "init_image".to_string(),
            image_id: "init_image_id".to_string(),
            name: "init_container_name".to_string(),
            ..Default::default()
        };

        let container_id = CONTAINER_ID;
        let result =
            get_container_identitifiers(&vec![container_status.clone()], container_id).unwrap();
        assert_eq!(result.name, container_status.name);
        assert_eq!(result.image, container_status.image);

        let container_id = "dummy_id";
        let result = get_container_identitifiers(&vec![container_status], container_id);
        assert!(result.is_none());
    }

    fn get_container_status() -> ContainerStatus {
        ContainerStatus {
            container_id: Some(format!("docker://{}", CONTAINER_ID)),
            image: "image".to_string(),
            image_id: "image_id".to_string(),
            name: "container_name".to_string(),
            ..ContainerStatus::default()
        }
    }

    #[test]
    fn container_status_match_container_id_happy_path() {
        let status = get_container_status();
        let container_id = CONTAINER_ID;
        let result = container_status_match_container_id(&status, container_id);
        assert!(result);
    }

    #[test]
    fn container_status_match_container_id_no_container_id() {
        let mut status = get_container_status();
        let container_id = CONTAINER_ID;

        //Remove container id
        status.container_id = None;
        let result = container_status_match_container_id(&status, container_id);
        assert!(!result);
    }

    #[test]
    fn container_status_match_container_id_bad_url() {
        let mut status = get_container_status();
        let container_id = CONTAINER_ID;

        // Badly form url
        status.container_id = Some("no url format".to_string());
        let result = container_status_match_container_id(&status, container_id);
        assert!(!result);
    }

    #[test]
    fn container_status_match_container_id_no_host() {
        let mut status = get_container_status();
        let container_id = CONTAINER_ID;

        // no host field
        status.container_id = Some("docker://".to_string());
        let result = container_status_match_container_id(&status, container_id);
        assert!(!result);
    }

    #[test]
    fn container_status_match_container_id_no_match() {
        let mut status = get_container_status();
        let container_id = CONTAINER_ID;

        // not a match
        status.container_id = Some(
            "docker://111111111111111111111111111111111111111111111111111111111111111".to_string(),
        );
        let result = container_status_match_container_id(&status, container_id);
        assert!(!result);
    }

    #[test]
    fn is_container_ready_in_pod_happy_path_match_container() {
        let pod = get_pods();
        let container_id = CONTAINER_ID;

        let result = is_container_ready_in_pod(&pod, container_id).unwrap();
        assert_eq!(result.image, "image");
        assert_eq!(result.name, "container_name");
    }

    #[test]
    fn is_container_ready_in_pod_happy_path_match_init_container() {
        let pod = get_pods();
        let container_id = INIT_CONTAINER_ID;

        let result = is_container_ready_in_pod(&pod, container_id).unwrap();
        assert_eq!(result.image, "init_image");
        assert_eq!(result.name, "init_container_name");
    }

    #[test]
    fn is_container_ready_in_pod_no_pod_name() {
        let mut pod = get_pods();
        let container_id = CONTAINER_ID;

        // Remove name
        pod.metadata.name = None;

        assert!(is_container_ready_in_pod(&pod, container_id).is_none());
    }

    #[test]
    fn is_container_ready_in_pod_no_pod_status() {
        let mut pod = get_pods();
        let container_id = CONTAINER_ID;

        // Remove status
        pod.status = None;

        assert!(is_container_ready_in_pod(&pod, container_id).is_none());
    }

    #[test]
    fn is_container_ready_in_pod_no_match() {
        let pod = get_pods();

        // Dummy id to avoid matching
        let container_id = "dummy";

        assert!(is_container_ready_in_pod(&pod, container_id).is_none());
    }

    #[tokio::test]
    async fn get_container_id_and_pod_uid_from_cgroup_happy_path() {
        let workload_attestation = init_selector_test().await;

        let mut cgroups = HashMap::new();
        let path = format!(
            "/docker/{}/kubepods/besteffort/pod{}/{}",
            CONTAINER_ID, POD_UID, CONTAINER_ID
        );
        cgroups.insert("pids".to_string(), path);

        let (container_id, pod_uid) = workload_attestation
            .get_container_id_and_pod_uid_from_cgroup(&cgroups)
            .unwrap();
        assert_eq!(container_id, CONTAINER_ID);
        assert_eq!(pod_uid, POD_UID);
    }

    #[tokio::test]
    async fn get_container_id_and_pod_uid_from_cgroup_error_no_pid_cgroup() {
        let workload_attestation = init_selector_test().await;

        let mut cgroups = HashMap::new();
        let path = format!(
            "/docker/{}/kubepods/besteffort/pod{}/{}",
            CONTAINER_ID, POD_UID, CONTAINER_ID
        );
        cgroups.insert("dummy".to_string(), path);

        let error = workload_attestation
            .get_container_id_and_pod_uid_from_cgroup(&cgroups)
            .unwrap_err();
        assert_matches!(error, Error::NoPIDcgroup);
    }

    #[tokio::test]
    async fn get_container_id_and_pod_uid_from_cgroup_error_cannot_extract_info() {
        let workload_attestation = init_selector_test().await;

        let mut cgroups = HashMap::new();
        let path = "dummy".to_string();
        cgroups.insert("pids".to_string(), path);

        let error = workload_attestation
            .get_container_id_and_pod_uid_from_cgroup(&cgroups)
            .unwrap_err();
        assert_matches!(error, Error::ExtractPodUIDandContainerID(_));
    }

    #[tokio::test]
    async fn get_pod_happy_path() {
        let mut workload_attestation = init_selector_test().await;

        let pod1 = get_pods();
        let pod2 = get_pods();

        let pod_list = ObjectList {
            metadata: ListMeta::default(),
            items: vec![pod1, pod2.clone()],
        };
        workload_attestation.client.queue_response(pod_list).await;

        let (pod, container_identifiers) = workload_attestation
            .get_pod(CONTAINER_ID, POD_UID)
            .await
            .unwrap();
        assert_eq!(pod, pod2);
        assert_eq!(container_identifiers.name, "container_name");
        assert_eq!(container_identifiers.image, "image");
    }

    #[tokio::test]
    async fn get_pod_error_listing_pods() {
        let mut workload_attestation = init_selector_test().await;

        workload_attestation.client.queue_response("dummy").await;

        let error = workload_attestation
            .get_pod(CONTAINER_ID, POD_UID)
            .await
            .unwrap_err();
        assert_matches!(
            error,
            Error::ListingPods {
                error: _,
                node_name: _
            }
        );
    }

    #[tokio::test]
    async fn get_pod_error_pod_not_found() {
        let mut workload_attestation = init_selector_test().await;

        let mut pod1 = get_pods();
        let mut pod2 = get_pods();

        pod1.status = None;
        pod2.status = None;

        // get pod will try and wait for status to be ready
        let mut count = 0;
        loop {
            let pod_list = ObjectList {
                metadata: ListMeta::default(),
                items: vec![pod1.clone(), pod2.clone()],
            };

            workload_attestation.client.queue_response(pod_list).await;
            count += 1;
            if count == workload_attestation.max_poll_attempt {
                break;
            }
        }

        let error = workload_attestation
            .get_pod(CONTAINER_ID, POD_UID)
            .await
            .unwrap_err();
        assert_matches!(
            error,
            Error::ContainerNotFoundInPod {
                container_id: _,
                pod_uid: _
            }
        );
    }

    #[tokio::test]
    async fn get_selector_info_happy_path() {
        let container_identifiers = ContainerIdentifiers {
            name: "name".to_string(),
            image: "image".to_string(),
        };

        let pod = get_pods();

        // No need to test the return. Already tested in main function happy path.
        get_selector_info(pod, container_identifiers).unwrap();
    }

    #[test]
    fn get_selector_info_error_no_spec() {
        let container_identifiers = ContainerIdentifiers {
            name: "name".to_string(),
            image: "image".to_string(),
        };

        let mut pod = get_pods();
        pod.spec = None;

        // No need to test the return. Already tested in main function happy path.
        let error = get_selector_info(pod, container_identifiers).unwrap_err();
        if let Error::MissingField(error) = error {
            assert_matches!(error, MissingField::PodSpec);
        } else {
            panic!("Bad error type");
        }
    }

    #[test]
    fn get_selector_info_error_no_status() {
        let container_identifiers = ContainerIdentifiers {
            name: "name".to_string(),
            image: "image".to_string(),
        };

        let mut pod = get_pods();
        pod.status = None;

        // No need to test the return. Already tested in main function happy path.
        let error = get_selector_info(pod, container_identifiers).unwrap_err();
        if let Error::MissingField(error) = error {
            assert_matches!(error, MissingField::Status);
        } else {
            panic!("Bad error type");
        }
    }

    #[test]
    fn get_selector_info_error_no_pod_name() {
        let container_identifiers = ContainerIdentifiers {
            name: "name".to_string(),
            image: "image".to_string(),
        };

        let mut pod = get_pods();
        pod.metadata.name = None;

        // No need to test the return. Already tested in main function happy path.
        let error = get_selector_info(pod, container_identifiers).unwrap_err();
        if let Error::MissingField(error) = error {
            assert_matches!(error, MissingField::PodName);
        } else {
            panic!("Bad error type");
        }
    }

    #[test]
    fn get_selector_info_error_no_pod_uid() {
        let container_identifiers = ContainerIdentifiers {
            name: "name".to_string(),
            image: "image".to_string(),
        };

        let mut pod = get_pods();
        pod.metadata.uid = None;

        // No need to test the return. Already tested in main function happy path.
        let error = get_selector_info(pod, container_identifiers).unwrap_err();
        if let Error::MissingField(error) = error {
            assert_matches!(error, MissingField::PodUid);
        } else {
            panic!("Bad error type");
        }
    }

    #[test]
    fn get_selector_info_error_no_namespace() {
        let container_identifiers = ContainerIdentifiers {
            name: "name".to_string(),
            image: "image".to_string(),
        };

        let mut pod = get_pods();
        pod.metadata.namespace = None;

        // No need to test the return. Already tested in main function happy path.
        let error = get_selector_info(pod, container_identifiers).unwrap_err();
        if let Error::MissingField(error) = error {
            assert_matches!(error, MissingField::Namespace);
        } else {
            panic!("Bad error type");
        }
    }

    #[test]
    fn get_selector_info_error_no_label() {
        let container_identifiers = ContainerIdentifiers {
            name: "name".to_string(),
            image: "image".to_string(),
        };

        let mut pod = get_pods();
        pod.metadata.labels = None;

        // No need to test the return. Already tested in main function happy path.
        let error = get_selector_info(pod, container_identifiers).unwrap_err();
        if let Error::MissingField(error) = error {
            assert_matches!(error, MissingField::PodLabels);
        } else {
            panic!("Bad error type");
        }
    }

    #[test]
    fn get_selector_info_error_no_node_name() {
        let container_identifiers = ContainerIdentifiers {
            name: "name".to_string(),
            image: "image".to_string(),
        };

        let mut pod = get_pods();
        if let Some(spec) = &mut pod.spec {
            spec.node_name = None;
        }

        // No need to test the return. Already tested in main function happy path.
        let error = get_selector_info(pod, container_identifiers).unwrap_err();
        if let Error::MissingField(error) = error {
            assert_matches!(error, MissingField::NodeName);
        } else {
            panic!("Bad error type");
        }
    }

    #[test]
    fn get_selector_info_error_no_service_account_name() {
        let container_identifiers = ContainerIdentifiers {
            name: "name".to_string(),
            image: "image".to_string(),
        };

        let mut pod = get_pods();
        if let Some(spec) = &mut pod.spec {
            spec.service_account_name = None;
        }

        // No need to test the return. Already tested in main function happy path.
        let error = get_selector_info(pod, container_identifiers).unwrap_err();
        if let Error::MissingField(error) = error {
            assert_matches!(error, MissingField::ServiceAccountName);
        } else {
            panic!("Bad error type");
        }
    }
}
