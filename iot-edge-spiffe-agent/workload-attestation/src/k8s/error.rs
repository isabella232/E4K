// Copyright (c) Microsoft. All rights reserved.
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Error while reading response from kube API, missing field {0}")]
    MissingField(MissingField),
    #[error("Could not get cgroups from PID {0}")]
    CgroupsFromPID(#[from] cgroups_rs::error::Error),
    #[error("No PID cgroup returned from the list")]
    NoPIDcgroup,
    #[error("Regex failed to extract POD UID and container ID {0}")]
    ExtractPodUIDandContainerID(String),
    #[error("Error while listing pod {error:?} from node {node_name:?}")]
    ListingPods {
        error: kube::error::Error,
        node_name: String,
    },
    #[error("Container id {container_id:?} not found in pod {pod_uid:?}")]
    ContainerNotFoundInPod {
        container_id: String,
        pod_uid: String,
    },
}

#[derive(Error, Debug)]
pub enum MissingField {
    #[error("Pod name")]
    PodName,
    #[error("Pod Uid")]
    PodUid,
    #[error("Namespace")]
    Namespace,
    #[error("Service account name")]
    ServiceAccountName,
    #[error("Pod spec")]
    PodSpec,
    #[error("Node Name")]
    NodeName,
    #[error("Pod labels")]
    PodLabels,
    #[error("Status")]
    Status,
}
