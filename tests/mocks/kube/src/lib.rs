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
use k8s_openapi::api::{
    authentication::v1::{TokenReview, TokenReviewSpec, TokenReviewStatus, UserInfo},
    core::v1::{Node, Pod, PodSpec, PodStatus},
};
use kube::{core::ObjectMeta, Error, Resource};
use serde::{de::DeserializeOwned, Serialize};
use std::{
    collections::{BTreeMap, VecDeque},
    marker::PhantomData,
    sync::{Arc, Mutex},
};

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
}

pub fn get_token_review_status() -> TokenReviewStatus {
    let mut token_review_status = TokenReviewStatus::default();

    let mut extra = BTreeMap::new();
    extra.insert(
        "authentication.kubernetes.io/pod-name".to_string(),
        ["pod_name".to_string()].to_vec(),
    );
    extra.insert(
        "authentication.kubernetes.io/pod-uid".to_string(),
        ["75dbabec-9510-11ec-b909-0242ac120002".to_string()].to_vec(),
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
    let mut pod_labels = BTreeMap::new();

    pod_spec.node_name = Some("node_name".to_string());
    pod_spec.service_account_name = Some("iotedge-spiffe-agent".to_string());
    pod.spec = Some(pod_spec);

    pod.metadata = ObjectMeta::default();
    pod.metadata.namespace = Some("namespace".to_string());
    pod_labels.insert("pod-name".to_string(), "pod".to_string());
    pod_labels.insert("shoudbefiltered".to_string(), "shoudbefiltered".to_string());
    pod.metadata.labels = Some(pod_labels);

    pod_status.host_ip = Some("127.0.0.1".to_string());
    pod.status = Some(pod_status);

    pod
}

pub fn get_nodes() -> Node {
    let mut node = Node::default();
    let mut node_labels = BTreeMap::new();

    node.metadata.uid = Some("14b57414-9516-11ec-b909-0242ac120002".to_string());
    node_labels.insert("node-name".to_string(), "node".to_string());
    node_labels.insert("shoudbefiltered".to_string(), "shoudbefiltered".to_string());
    node.metadata.labels = Some(node_labels);

    node
}
