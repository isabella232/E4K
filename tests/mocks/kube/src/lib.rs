// Copyright (c) Microsoft. All rights reserved.

#![deny(rust_2018_idioms)]
#![warn(clippy::all, clippy::pedantic)]
#![allow(
    clippy::default_trait_access,
    clippy::let_unit_value,
    clippy::missing_errors_doc,
    clippy::similar_names,
    clippy::too_many_lines,
    clippy::needless_pass_by_value,
    clippy::missing_panics_doc,
    clippy::must_use_candidate
)]

use core::fmt::Debug;
use http::Request;
use k8s_openapi::{
    api::{
        authentication::v1::{TokenReview, TokenReviewSpec, TokenReviewStatus, UserInfo},
        core::v1::{Container, ContainerStatus, Node, Pod, PodSpec, PodStatus},
    },
    apimachinery::pkg::apis::meta::v1::OwnerReference,
};
use kube::{
    api::ListParams,
    core::{ObjectList, ObjectMeta},
    Error, Resource,
};
use serde::{de::DeserializeOwned, Serialize};
use std::{
    collections::{BTreeMap, VecDeque},
    marker::PhantomData,
    sync::{Arc, Mutex},
};

pub const POD_UID: &str = "75dbabec-9510-11ec-b909-0242ac120002";
pub const CONTAINER_ID: &str = "cbb8bd346ba774d1a67d622cd7a96d3bfbb98719b30918786ac5ea5eb84807b3";
pub const INIT_CONTAINER_ID: &str = "11111111111111111111111111111111111111111111111111111111";
pub const NODE_UID: &str = "14b57414-9516-11ec-b909-0242ac120002";

#[derive(Clone)]
pub struct Client {
    response_queue: Arc<Mutex<VecDeque<String>>>,
}

impl Client {
    pub async fn try_default() -> Result<Self, Error> {
        Ok(Self {
            response_queue: Arc::new(Mutex::new(VecDeque::new())),
        })
    }

    pub fn new<S, B, T>(_service: S, _default_namespace: T) -> Self {
        Self {
            response_queue: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    pub async fn queue_response<T>(&mut self, request: T)
    where
        T: Serialize,
    {
        let mut response_queue = self.response_queue.lock().unwrap();
        response_queue.push_back(serde_json::to_string(&request).unwrap());
    }

    pub async fn request<T>(&self, _request: Request<Vec<u8>>) -> Result<T, Error>
    where
        T: DeserializeOwned,
    {
        let mut response_queue = self.response_queue.lock().unwrap();
        let request_response = response_queue.pop_front().unwrap();
        serde_json::from_str(&request_response).map_err(Error::SerdeError)
    }
}

pub struct Api<K> {
    client: Client,
    phantom: PhantomData<K>,
}

impl<K: Resource> Api<K>
where
    <K as Resource>::DynamicType: Default,
{
    pub fn all(client: Client) -> Self {
        Self {
            client,
            phantom: PhantomData,
        }
    }

    pub fn namespaced(client: Client, _ns: &str) -> Self {
        Self {
            client,
            phantom: PhantomData,
        }
    }

    pub fn default_namespaced(client: Client) -> Self {
        Self {
            client,
            phantom: PhantomData,
        }
    }
}

impl<K: Resource> Api<K>
where
    K: Clone + DeserializeOwned + Debug,
{
    pub async fn get(&self, _name: &str) -> Result<K, Error> {
        let req = Request::default();
        self.client.request::<K>(req).await
    }

    pub async fn list(&self, _list_params: &ListParams) -> Result<ObjectList<K>, Error> {
        let req = Request::default();
        self.client.request::<ObjectList<K>>(req).await
    }
}

pub fn get_token_review_status() -> TokenReviewStatus {
    let mut token_review_status = TokenReviewStatus::default();

    let mut extra = BTreeMap::new();
    extra.insert(
        "authentication.kubernetes.io/pod-name".to_string(),
        vec!["pod_name".to_string()],
    );
    extra.insert(
        "authentication.kubernetes.io/pod-uid".to_string(),
        vec![POD_UID.to_string()],
    );
    let user_info = UserInfo {
        extra: Some(extra),
        ..Default::default()
    };

    token_review_status.authenticated = Some(true);
    token_review_status.user = Some(user_info);

    token_review_status
}

pub fn get_token_review() -> TokenReview {
    let mut token_review = TokenReview::default();
    let mut token_review_status = get_token_review_status();

    token_review_status.authenticated = Some(true);

    token_review.metadata = ObjectMeta::default();
    token_review.spec = TokenReviewSpec::default();
    token_review.status = Some(token_review_status);

    token_review
}

pub fn get_pods() -> Pod {
    let mut pod = Pod::default();
    let mut pod_status = PodStatus::default();
    let mut pod_spec = PodSpec::default();
    let mut container_status = ContainerStatus::default();
    let mut pod_labels = BTreeMap::new();
    let mut pod_owner = OwnerReference::default();
    let mut container = Container::default();
    let mut init_container = Container::default();
    let init_container_status = ContainerStatus {
        container_id: Some(format!("docker://{}", INIT_CONTAINER_ID)),
        image: "init_image".to_string(),
        image_id: "init_image_id".to_string(),
        name: "init_container_name".to_string(),
        ..Default::default()
    };

    init_container.image = Some("debian:latest".to_string());
    container.image = Some("ubuntu:latest".to_string());

    pod_owner.uid = "uid".to_string();
    pod_owner.name = "name".to_string();
    pod_owner.kind = "kind".to_string();

    container_status.container_id = Some(format!("docker://{}", CONTAINER_ID));
    container_status.image = "image".to_string();
    container_status.image_id = "image_id".to_string();
    container_status.name = "container_name".to_string();

    pod_spec.init_containers = Some(vec![init_container]);
    pod_spec.containers = vec![container];
    pod_spec.node_name = Some("node_name".to_string());
    pod_spec.service_account_name = Some("iotedge-spiffe-agent".to_string());
    pod.spec = Some(pod_spec);

    pod.metadata = ObjectMeta::default();
    pod.metadata.name = Some("pod_name".to_string());
    pod.metadata.namespace = Some("namespace".to_string());
    pod.metadata.uid = Some(POD_UID.to_string());
    pod.metadata.owner_references = Some(vec![pod_owner]);

    pod_labels.insert("pod-name".to_string(), "pod".to_string());
    pod_labels.insert("shoudbefiltered".to_string(), "shoudbefiltered".to_string());
    pod.metadata.labels = Some(pod_labels);

    pod_status.host_ip = Some("127.0.0.1".to_string());
    pod_status.container_statuses = Some(vec![container_status]);
    pod_status.init_container_statuses = Some(vec![init_container_status]);
    pod.status = Some(pod_status);

    pod
}

pub fn get_nodes() -> Node {
    let mut node = Node::default();
    let mut node_labels = BTreeMap::new();

    node.metadata.uid = Some(NODE_UID.to_string());
    node_labels.insert("node-name".to_string(), "node".to_string());
    node_labels.insert("shoudbefiltered".to_string(), "shoudbefiltered".to_string());
    node.metadata.labels = Some(node_labels);

    node
}
