// Copyright (c) Microsoft. All rights reserved.

pub mod error;

use std::collections::{BTreeMap, BTreeSet};

use core_objects::{build_selector_string, NodeSelectorType};
use k8s_openapi::api::{
    authentication::v1::{TokenReview, TokenReviewStatus},
    core::v1::{Node, Pod},
};

#[cfg(not(any(test, feature = "tests")))]
use kube::{Api, Client};
#[cfg(any(test, feature = "tests"))]
use mock_kube::{Api, Client};

use log::{debug, info};
use server_config::NodeAttestationConfigPsat;

use crate::{psat::error::MissingField, AgentAttributes, NodeAttestation as NodeAttestationTrait};

use error::Error;

#[derive(Clone, Debug, Default)]
struct SelectorInfo {
    cluster_name: String,
    namespace: String,
    service_account_name: String,
    pod_name: String,
    pod_uid: String,
    node_ip: String,
    node_name: String,
    node_uid: String,
    node_labels: BTreeMap<String, String>,
    pod_labels: BTreeMap<String, String>,
}

pub struct NodeAttestation {
    service_account_allow_list: BTreeSet<String>,
    audience: String,
    allowed_node_label_keys: BTreeSet<String>,
    allowed_pod_label_keys: BTreeSet<String>,
    cluster_name: String,
    client: Client,
}

impl NodeAttestation {
    #[must_use]
    pub fn new(config: &NodeAttestationConfigPsat, client: Client) -> Self {
        NodeAttestation {
            service_account_allow_list: config.service_account_allow_list.clone(),
            audience: config.audience.clone(),
            allowed_node_label_keys: config.allowed_node_label_keys.clone(),
            allowed_pod_label_keys: config.allowed_pod_label_keys.clone(),
            cluster_name: config.cluster_name.clone(),
            client,
        }
    }

    async fn review_token(&self, token: &str) -> Result<TokenReviewStatus, Error> {
        let mut body = TokenReview::default();
        let _ = body.spec.token.insert(token.to_string());
        let _ = body.spec.audiences = Some(vec![self.audience.clone()]);

        let (req, _) = TokenReview::create_token_review(&body, Default::default())
            .map_err(Error::TokenReviewRequest)?;

        let resp = self
            .client
            .request::<TokenReview>(req)
            .await
            .map_err(Error::K8sTokenReviewAPI)?;

        let token_review_status = resp
            .status
            .ok_or(Error::MissingField(MissingField::TokenReviewStatus))?;

        token_review_status
            .authenticated
            .ok_or(Error::MissingField(MissingField::Authenticated))?
            .then(|| ())
            .ok_or_else(|| {
                if let Some(error) = token_review_status.error.clone() {
                    Error::InvalidToken(error)
                } else {
                    Error::InvalidToken(String::new())
                }
            })?;

        Ok(token_review_status)
    }

    async fn get_selector_info(
        &self,
        token_review_status: TokenReviewStatus,
    ) -> Result<SelectorInfo, Error> {
        let extras = token_review_status
            .user
            .ok_or(Error::MissingField(MissingField::UserInfo))?
            .extra
            .ok_or(Error::MissingField(MissingField::Extra))?;

        let pod_name = extras
            .get("authentication.kubernetes.io/pod-name")
            .ok_or(Error::MissingField(MissingField::PodName))?
            .first()
            .ok_or(Error::MissingField(MissingField::PodName))?
            .clone();

        let pods: Api<Pod> = Api::default_namespaced(self.client.clone());

        let pod = pods.get(&pod_name).await.map_err(Error::GettingPodInfo)?;

        let pod_spec = pod.spec.ok_or(Error::MissingField(MissingField::PodSpec))?;
        let pod_status = pod
            .status
            .ok_or(Error::MissingField(MissingField::PodStatus))?;

        let node_name = pod_spec
            .node_name
            .ok_or(Error::MissingField(MissingField::NodeName))?;
        let nodes: Api<Node> = Api::all(self.client.clone());

        let node = nodes
            .get(&node_name)
            .await
            .map_err(Error::GettingNodeInfo)?;

        let selector_info = SelectorInfo {
            cluster_name: self.cluster_name.clone(),
            pod_name,
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
                .ok_or(Error::MissingField(MissingField::PodLabels))?
                .into_iter()
                .filter(|(key, _)| self.allowed_pod_label_keys.get(key).is_some())
                .collect(),
            node_name,
            service_account_name: pod_spec
                .service_account_name
                .ok_or(Error::MissingField(MissingField::ServiceAccountName))?,
            node_ip: pod_status
                .host_ip
                .ok_or(Error::MissingField(MissingField::HostIP))?,
            node_uid: node
                .metadata
                .uid
                .ok_or(Error::MissingField(MissingField::NodeUid))?,
            node_labels: node
                .metadata
                .labels
                .ok_or(Error::MissingField(MissingField::NodeLabels))?
                .into_iter()
                .filter(|(key, _)| self.allowed_node_label_keys.get(key).is_some())
                .collect(),
        };

        self.service_account_allow_list
            .get(&selector_info.service_account_name)
            .is_some()
            .then(|| ())
            .ok_or_else(|| {
                Error::ServiceAccountNotAllowed(selector_info.service_account_name.clone())
            })?;

        Ok(selector_info)
    }

    async fn auth_agent(&self, token: &str) -> Result<AgentAttributes, Error> {
        let token_review_status = self.review_token(token).await?;

        let selector_info = self.get_selector_info(token_review_status).await?;

        let mut selectors = BTreeSet::new();
        selectors.insert(build_selector_string(
            &NodeSelectorType::Cluster,
            &selector_info.cluster_name,
        ));
        selectors.insert(build_selector_string(
            &NodeSelectorType::AgentNameSpace,
            &selector_info.namespace,
        ));
        selectors.insert(build_selector_string(
            &NodeSelectorType::AgentServiceAccount,
            &selector_info.service_account_name,
        ));
        selectors.insert(build_selector_string(
            &NodeSelectorType::AgentPodName,
            &selector_info.pod_name,
        ));
        selectors.insert(build_selector_string(
            &NodeSelectorType::AgentPodUID,
            &selector_info.pod_uid,
        ));
        selectors.insert(build_selector_string(
            &NodeSelectorType::AgentNodeIP,
            &selector_info.node_ip,
        ));
        selectors.insert(build_selector_string(
            &NodeSelectorType::AgentNodeName,
            &selector_info.node_name,
        ));
        selectors.insert(build_selector_string(
            &NodeSelectorType::AgentNodeUID,
            &selector_info.node_uid,
        ));

        push_map_into_selectors(
            &mut selectors,
            &selector_info.node_labels,
            &NodeSelectorType::AgentNodeLabels,
        );
        push_map_into_selectors(
            &mut selectors,
            &selector_info.pod_labels,
            &NodeSelectorType::AgentPodLabels,
        );

        info!(
            "IoTEdge SPIFFE Agent {} was attested successfully",
            selector_info.pod_name
        );
        debug!("Found the following selectors for workload {:?}", selectors);

        Ok(AgentAttributes { selectors })
    }
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
        let selector = build_selector_string(&selector_type, map);
        selectors.insert(selector);
    }
}

#[async_trait::async_trait]
impl NodeAttestationTrait for NodeAttestation {
    async fn attest_agent(
        &self,
        token: &str,
    ) -> Result<AgentAttributes, Box<dyn std::error::Error + Send>> {
        self.auth_agent(token)
            .await
            .map_err(|err| Box::new(err) as _)
    }
}

#[cfg(test)]
mod tests {
    use core_objects::CONFIG_DEFAULT_PATH;
    use matches::assert_matches;
    use mock_kube::{get_nodes, get_pods, get_token_review, get_token_review_status};
    use server_config::Config;

    use super::*;

    async fn init_selector_test() -> NodeAttestation {
        let config = Config::load_config(CONFIG_DEFAULT_PATH).unwrap();

        let node_attestation_config = match config.node_attestation_config {
            server_config::NodeAttestationConfig::Sat(_) => panic!("Unexpected type"),
            server_config::NodeAttestationConfig::Psat(psat) => psat,
        };

        let client = Client::try_default().await.unwrap();
        NodeAttestation::new(&node_attestation_config, client)
    }

    #[tokio::test]
    async fn auth_agent_happy_path() {
        let mut node_attestation = init_selector_test().await;

        let pod = get_pods();
        let node = get_nodes();
        let token_review = get_token_review();

        node_attestation.client.queue_response(token_review).await;
        node_attestation.client.queue_response(pod).await;
        node_attestation.client.queue_response(node).await;

        let resp = node_attestation.auth_agent("dummy token").await.unwrap();

        let cluster_selector = build_selector_string(&NodeSelectorType::Cluster, "demo-cluster");
        assert!(resp.selectors.contains(&cluster_selector));

        let namespace_selector =
            build_selector_string(&NodeSelectorType::AgentNameSpace, "namespace");
        assert!(resp.selectors.contains(&namespace_selector));

        let service_account = build_selector_string(
            &NodeSelectorType::AgentServiceAccount,
            "iotedge-spiffe-agent",
        );
        assert!(resp.selectors.contains(&service_account));

        let pod_name = build_selector_string(&NodeSelectorType::AgentPodName, "pod_name");
        assert!(resp.selectors.contains(&pod_name));

        let pod_uid = build_selector_string(
            &NodeSelectorType::AgentPodUID,
            "75dbabec-9510-11ec-b909-0242ac120002",
        );
        assert!(resp.selectors.contains(&pod_uid));

        let node_ip = build_selector_string(&NodeSelectorType::AgentNodeIP, "127.0.0.1");
        assert!(resp.selectors.contains(&node_ip));

        let node_name = build_selector_string(&NodeSelectorType::AgentNodeName, "node_name");
        assert!(resp.selectors.contains(&node_name));

        let node_uid = build_selector_string(
            &NodeSelectorType::AgentNodeUID,
            "14b57414-9516-11ec-b909-0242ac120002",
        );
        assert!(resp.selectors.contains(&node_uid));

        let pod_labels = build_selector_string(
            &NodeSelectorType::AgentPodLabels,
            &format!("{}:{}", "pod-name", "pod"),
        );
        assert!(resp.selectors.contains(&pod_labels));

        let node_labels = build_selector_string(
            &NodeSelectorType::AgentNodeLabels,
            &format!("{}:{}", "node-name", "node"),
        );
        assert!(resp.selectors.contains(&node_labels));
    }

    #[tokio::test]
    async fn get_selector_happy_path() {
        let mut node_attestation = init_selector_test().await;

        let pod = get_pods();
        let node = get_nodes();
        let token_review_status = get_token_review_status();

        node_attestation.client.queue_response(pod).await;
        node_attestation.client.queue_response(node).await;

        let selector_info = node_attestation
            .get_selector_info(token_review_status)
            .await
            .unwrap();

        assert_eq!(selector_info.cluster_name, "demo-cluster");
        assert_eq!(selector_info.namespace, "namespace");
        assert_eq!(selector_info.service_account_name, "iotedge-spiffe-agent");
        assert_eq!(selector_info.pod_name, "pod_name");
        assert_eq!(
            selector_info.pod_uid,
            "75dbabec-9510-11ec-b909-0242ac120002"
        );
        assert_eq!(selector_info.node_ip, "127.0.0.1");
        assert_eq!(selector_info.node_name, "node_name");
        assert_eq!(
            selector_info.node_uid,
            "14b57414-9516-11ec-b909-0242ac120002"
        );
        assert_eq!(selector_info.node_labels.len(), 1);
        assert_eq!(selector_info.pod_labels.len(), 1);
        assert_eq!(selector_info.pod_labels["pod-name"], "pod");
        assert_eq!(selector_info.node_labels["node-name"], "node");
    }

    #[tokio::test]
    async fn get_selector_service_account_not_allowed_error() {
        let mut node_attestation = init_selector_test().await;
        let mut pod = get_pods();
        if let Some(spec) = &mut pod.spec {
            let service_account_name = "ForbiddenServiceAccount".to_string();
            spec.service_account_name = Some(service_account_name);
        }
        let node = get_nodes();

        let token_review_status = get_token_review_status();

        node_attestation.client.queue_response(pod).await;
        node_attestation.client.queue_response(node).await;

        let error = node_attestation
            .get_selector_info(token_review_status)
            .await
            .unwrap_err();

        assert_matches!(error, Error::ServiceAccountNotAllowed(_));
    }

    #[tokio::test]
    async fn get_selector_node_labels_error() {
        let mut node_attestation = init_selector_test().await;

        let pod = get_pods();
        let mut node = get_nodes();
        node.metadata.labels = None;
        let token_review_status = get_token_review_status();

        node_attestation.client.queue_response(pod).await;
        node_attestation.client.queue_response(node).await;

        let error = node_attestation
            .get_selector_info(token_review_status)
            .await
            .unwrap_err();

        assert_matches!(error, Error::MissingField(MissingField::NodeLabels));
    }

    #[tokio::test]
    async fn get_selector_pod_labels_error() {
        let mut node_attestation = init_selector_test().await;

        let mut pod = get_pods();
        pod.metadata.labels = None;
        let node = get_nodes();
        let token_review_status = get_token_review_status();

        node_attestation.client.queue_response(pod).await;
        node_attestation.client.queue_response(node).await;

        let error = node_attestation
            .get_selector_info(token_review_status)
            .await
            .unwrap_err();

        assert_matches!(error, Error::MissingField(MissingField::PodLabels));
    }

    #[tokio::test]
    async fn get_selector_node_uid_error() {
        let mut node_attestation = init_selector_test().await;

        let pod = get_pods();
        let mut node = get_nodes();
        node.metadata.uid = None;
        let token_review_status = get_token_review_status();

        node_attestation.client.queue_response(pod).await;
        node_attestation.client.queue_response(node).await;

        let error = node_attestation
            .get_selector_info(token_review_status)
            .await
            .unwrap_err();

        assert_matches!(error, Error::MissingField(MissingField::NodeUid));
    }

    #[tokio::test]
    async fn get_selector_host_ip_error() {
        let mut node_attestation = init_selector_test().await;

        let mut pod = get_pods();
        if let Some(status) = &mut pod.status {
            status.host_ip = None;
        }
        let node = get_nodes();
        let token_review_status = get_token_review_status();

        node_attestation.client.queue_response(pod).await;
        node_attestation.client.queue_response(node).await;

        let error = node_attestation
            .get_selector_info(token_review_status)
            .await
            .unwrap_err();

        assert_matches!(error, Error::MissingField(MissingField::HostIP));
    }

    #[tokio::test]
    async fn get_selector_pod_status_error() {
        let mut node_attestation = init_selector_test().await;

        let mut pod = get_pods();
        pod.status = None;
        let node = get_nodes();
        let token_review_status = get_token_review_status();

        node_attestation.client.queue_response(pod).await;
        node_attestation.client.queue_response(node).await;

        let error = node_attestation
            .get_selector_info(token_review_status)
            .await
            .unwrap_err();

        assert_matches!(error, Error::MissingField(MissingField::PodStatus));
    }

    #[tokio::test]
    async fn get_selector_service_account_name_error() {
        let mut node_attestation = init_selector_test().await;

        let mut pod = get_pods();
        if let Some(spec) = &mut pod.spec {
            spec.service_account_name = None;
        }
        let node = get_nodes();
        let token_review_status = get_token_review_status();

        node_attestation.client.queue_response(pod).await;
        node_attestation.client.queue_response(node).await;

        let error = node_attestation
            .get_selector_info(token_review_status)
            .await
            .unwrap_err();

        assert_matches!(error, Error::MissingField(MissingField::ServiceAccountName));
    }

    #[tokio::test]
    async fn get_selector_namespace_error() {
        let mut node_attestation = init_selector_test().await;

        let mut pod = get_pods();
        pod.metadata.namespace = None;
        let node = get_nodes();
        let token_review_status = get_token_review_status();

        node_attestation.client.queue_response(pod).await;
        node_attestation.client.queue_response(node).await;

        let error = node_attestation
            .get_selector_info(token_review_status)
            .await
            .unwrap_err();

        assert_matches!(error, Error::MissingField(MissingField::Namespace));
    }

    #[tokio::test]
    async fn get_selector_getting_node_info_error() {
        let mut node_attestation = init_selector_test().await;

        let pod = get_pods();
        let token_review_status = get_token_review_status();

        node_attestation.client.queue_response(pod).await;
        node_attestation.client.queue_response("{}").await;

        let error = node_attestation
            .get_selector_info(token_review_status)
            .await
            .unwrap_err();

        assert_matches!(error, Error::GettingNodeInfo(_));
    }

    #[tokio::test]
    async fn get_selector_user_info_error() {
        let mut node_attestation = init_selector_test().await;

        let pod = get_pods();
        let mut token_review_status = get_token_review_status();
        token_review_status.user = None;

        node_attestation.client.queue_response(pod).await;

        let error = node_attestation
            .get_selector_info(token_review_status)
            .await
            .unwrap_err();

        assert_matches!(error, Error::MissingField(MissingField::UserInfo));
    }

    #[tokio::test]
    async fn get_selector_extra_error() {
        let mut node_attestation = init_selector_test().await;

        let pod = get_pods();
        let mut token_review_status = get_token_review_status();
        if let Some(user) = &mut token_review_status.user {
            user.extra = None;
        }

        node_attestation.client.queue_response(pod).await;

        let error = node_attestation
            .get_selector_info(token_review_status)
            .await
            .unwrap_err();

        assert_matches!(error, Error::MissingField(MissingField::Extra));
    }

    #[tokio::test]
    async fn get_selector_pod_spec_error() {
        let mut node_attestation = init_selector_test().await;

        let mut pod = get_pods();
        pod.spec = None;
        let token_review_status = get_token_review_status();

        node_attestation.client.queue_response(pod).await;

        let error = node_attestation
            .get_selector_info(token_review_status)
            .await
            .unwrap_err();

        assert_matches!(error, Error::MissingField(MissingField::PodSpec));
    }

    #[tokio::test]
    async fn get_selector_node_name_error() {
        let mut node_attestation = init_selector_test().await;

        let mut pod = get_pods();
        if let Some(spec) = &mut pod.spec {
            spec.node_name = None;
        }

        let token_review_status = get_token_review_status();

        node_attestation.client.queue_response(pod).await;

        let error = node_attestation
            .get_selector_info(token_review_status)
            .await
            .unwrap_err();

        assert_matches!(error, Error::MissingField(MissingField::NodeName));
    }

    #[tokio::test]
    async fn get_selector_pod_name_error() {
        let mut node_attestation = init_selector_test().await;

        let pod = get_pods();
        let node = get_nodes();
        let mut token_review_status = get_token_review_status();
        if let Some(user) = &mut token_review_status.user {
            if let Some(extra) = &mut user.extra {
                extra
                    .remove("authentication.kubernetes.io/pod-name")
                    .unwrap();
            }
        }
        node_attestation.client.queue_response(pod).await;
        node_attestation.client.queue_response(node).await;

        let error = node_attestation
            .get_selector_info(token_review_status)
            .await
            .unwrap_err();

        assert_matches!(error, Error::MissingField(MissingField::PodName));
    }

    #[tokio::test]
    async fn get_selector_pod_uid_error() {
        let mut node_attestation = init_selector_test().await;

        let mut pod = get_pods();
        let node = get_nodes();
        let token_review_status = get_token_review_status();
        pod.metadata.uid = None;

        node_attestation.client.queue_response(pod).await;
        node_attestation.client.queue_response(node).await;

        let error = node_attestation
            .get_selector_info(token_review_status)
            .await
            .unwrap_err();

        assert_matches!(error, Error::MissingField(MissingField::PodUid));
    }

    #[tokio::test]
    async fn review_token_test_happy_path() {
        let mut node_attestation = init_selector_test().await;

        let token_review = get_token_review();
        node_attestation.client.queue_response(token_review).await;

        node_attestation.review_token("dummy").await.unwrap();
    }

    #[tokio::test]
    async fn review_token_test_missing_token_review_status_error() {
        let mut node_attestation = init_selector_test().await;

        let mut token_review = get_token_review();
        token_review.status = None;
        node_attestation.client.queue_response(token_review).await;

        let error = node_attestation.review_token("dummy").await.unwrap_err();

        assert_matches!(error, Error::MissingField(MissingField::TokenReviewStatus));
    }

    #[tokio::test]
    async fn review_token_test_api_call_error() {
        let mut node_attestation = init_selector_test().await;

        node_attestation.client.queue_response("{}").await;

        let error = node_attestation.review_token("dummy").await.unwrap_err();

        assert_matches!(error, Error::K8sTokenReviewAPI(_));
    }

    #[tokio::test]
    async fn review_token_test_failed_auth_or_none_error() {
        let mut node_attestation = init_selector_test().await;

        // Check if status is missing
        let mut token_review = get_token_review();
        if let Some(status) = &mut token_review.status {
            status.authenticated = None;
        };

        node_attestation.client.queue_response(token_review).await;

        let error = node_attestation.review_token("dummy").await.unwrap_err();

        assert_matches!(error, Error::MissingField(MissingField::Authenticated));

        // Check if auth failed
        let mut token_review = get_token_review();
        if let Some(status) = &mut token_review.status {
            //failed auth
            status.authenticated = Some(false);
        };

        node_attestation.client.queue_response(token_review).await;

        let error = node_attestation.review_token("dummy").await.unwrap_err();

        assert_matches!(error, Error::InvalidToken(_));
    }
}
