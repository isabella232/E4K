// Copyright (c) Microsoft. All rights reserved.
use k8s_openapi::RequestError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Unable to create Kube Client {0}")]
    UnableToCreateKubeClient(kube::Error),
    #[error("Service account not allowed {0}")]
    ServiceAccountNotAllowed(String),
    #[error("Error while creating token review request {0}")]
    TokenReviewRequest(RequestError),
    #[error("Error while calling token review API {0}")]
    K8sTokenReviewAPI(kube::Error),
    #[error("K8s API failed to authenticate token {0}")]
    InvalidToken(String),
    #[error("Error while querying pod information to extract selectors {0}")]
    GettingPodInfo(kube::Error),
    #[error("Error while querying node information to extract selectors {0}")]
    GettingNodeInfo(kube::Error),
    #[error("Error while reading response from kube API, missing field {0}")]
    MissingField(MissingField),
    #[error("Failed to create agent jwt svid {0}")]
    AgentJWTSVID(svid_factory::error::Error),
    #[error("Failed to load selectors in catalog {0}")]
    SetSelectors(Box<dyn std::error::Error + Send>),
}

#[derive(Error, Debug)]
pub enum MissingField {
    #[error("User Info")]
    UserInfo,
    #[error("Extra")]
    Extra,
    #[error("Pod name")]
    PodName,
    #[error("Pod Uid")]
    PodUid,
    #[error("Cluster name")]
    ClusterName,
    #[error("Namespace")]
    Namespace,
    #[error("Service account name")]
    ServiceAccountName,
    #[error("Pod status")]
    PodStatus,
    #[error("Pod spec")]
    PodSpec,
    #[error("Node Name")]
    NodeName,
    #[error("Token review status")]
    TokenReviewStatus,
    #[error("Authenticated")]
    Authenticated,
    #[error("Host IP")]
    HostIP,
    #[error("Node Uid")]
    NodeUid,
    #[error("Pod labels")]
    PodLabels,
    #[error("Node labels")]
    NodeLabels,
}
