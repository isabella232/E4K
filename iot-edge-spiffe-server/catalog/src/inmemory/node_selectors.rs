// Copyright (c) Microsoft. All rights reserved.

use std::collections::HashMap;

use core_objects::{NodeSelector, SPIFFEID};

use crate::{NodeSelectors, SelectorType};

use super::{error::Error, Catalog};

#[async_trait::async_trait]
impl NodeSelectors for Catalog {
    async fn get_selectors(
        &self,
        spiffe_id: &SPIFFEID,
    ) -> Result<HashMap<SelectorType, NodeSelector>, Box<dyn std::error::Error + Send>> {
        let node_selector_store = self.node_selector_store.read();

        let selectors = node_selector_store
            .get(&spiffe_id.to_string())
            .ok_or_else(|| Box::new(Error::KeyNotFound(spiffe_id.to_string())) as _)
            .map(Clone::clone)?;

        Ok(selectors)
    }

    async fn set_selectors(
        &self,
        spiffe_id: &SPIFFEID,
        selectors: HashMap<SelectorType, NodeSelector>,
    ) -> Result<(), Box<dyn std::error::Error + Send>> {
        let mut node_selector_store = self.node_selector_store.write();

        node_selector_store.insert(spiffe_id.to_string(), selectors);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn init_selector_test() -> (Catalog, SPIFFEID, HashMap<SelectorType, NodeSelector>) {
        let spiffe_id = SPIFFEID {
            trust_domain: "trust_domain".to_string(),
            path: "path".to_string(),
        };
        let mut selectors = HashMap::new();
        selectors.insert(
            SelectorType::Cluster,
            NodeSelector::Cluster("dummy".to_string()),
        );
        selectors.insert(
            SelectorType::AgentNameSpace,
            NodeSelector::AgentNameSpace("dummy".to_string()),
        );

        (Catalog::new(), spiffe_id, selectors)
    }

    #[tokio::test]
    async fn set_get_selectors_test_happy_path() {
        let (catalog, spiffe_id, selectors) = init_selector_test();

        catalog
            .set_selectors(&spiffe_id, selectors.clone())
            .await
            .unwrap();

        let selectors_tmp = catalog.get_selectors(&spiffe_id).await.unwrap();

        assert_eq!(selectors, selectors_tmp);
    }
}
