// Copyright (c) Microsoft. All rights reserved.

use core_objects::RegistrationEntry;

use crate::Entries;

use super::{error::Error, Catalog};

#[async_trait::async_trait]
impl Entries for Catalog {
    async fn batch_create(
        &self,
        entries: Vec<RegistrationEntry>,
    ) -> Result<(), Vec<(String, Box<dyn std::error::Error + Send>)>> {
        let mut entries_list = self.entries_list.write();
        let mut errors = Vec::new();

        for entry in entries {
            if entries_list.contains_key(&entry.id) {
                let error = (
                    entry.id.clone(),
                    Box::new(Error::DuplicatedEntry(entry.id)) as _,
                );

                errors.push(error);
            } else {
                entries_list.insert(entry.id.clone(), entry);
            };
        }

        errors.is_empty().then(|| ()).ok_or(errors)
    }

    async fn batch_update(
        &self,
        entries: Vec<RegistrationEntry>,
    ) -> Result<(), Vec<(String, Box<dyn std::error::Error + Send>)>> {
        let mut entries_list = self.entries_list.write();
        let mut errors = Vec::new();

        for entry in entries {
            if let Some(entry_ptr) = entries_list.get_mut(&entry.id) {
                *entry_ptr = entry;
            } else {
                let error = (
                    entry.id.clone(),
                    Box::new(Error::EntryNotFound(entry.id.clone())) as _,
                );

                errors.push(error);
            };
        }

        errors.is_empty().then(|| ()).ok_or(errors)
    }

    async fn batch_delete(
        &self,
        ids: &[String],
    ) -> Result<(), Vec<(String, Box<dyn std::error::Error + Send>)>> {
        let mut entries_list = self.entries_list.write();
        let mut errors = Vec::new();

        for id in ids {
            if entries_list.remove(id).is_none() {
                let error = (
                    id.clone(),
                    Box::new(Error::EntryNotFound(id.to_string())) as _,
                );

                errors.push(error);
            };
        }

        errors.is_empty().then(|| ()).ok_or(errors)
    }

    async fn batch_get(
        &self,
        ids: &[String],
    ) -> Vec<(
        String,
        Result<RegistrationEntry, Box<dyn std::error::Error + Send>>,
    )> {
        let entries_list = self.entries_list.read();
        let mut results = Vec::new();

        for id in ids {
            let entry = entries_list.get(id);

            let result = if let Some(entry) = entry {
                (id.clone(), Ok(entry.clone()))
            } else {
                (
                    id.clone(),
                    Err(Box::new(Error::EntryNotFound(id.to_string())) as _),
                )
            };

            results.push(result);
        }

        results
    }

    async fn get_entry(
        &self,
        id: &str,
    ) -> Result<RegistrationEntry, Box<dyn std::error::Error + Send>> {
        let entries_list = self.entries_list.read();

        let entry = entries_list
            .get(id)
            .ok_or_else(|| Box::new(Error::EntryNotFound(id.to_string())) as _)?;

        Ok(entry.clone())
    }

    async fn list_all(
        &self,
        page_token: Option<String>,
        page_size: usize,
    ) -> Result<(Vec<RegistrationEntry>, Option<String>), Box<dyn std::error::Error + Send>> {
        let entries_list = self.entries_list.read();

        let mut response: Vec<RegistrationEntry> = Vec::new();
        let mut entry_counter = 0;

        if page_size == 0 {
            return Err(Box::new(Error::InvalidPageSize()));
        }

        let mut iterator: Box<dyn Iterator<Item = (&String, &RegistrationEntry)>> =
            if let Some(page_token) = page_token {
                Box::new(entries_list.range(page_token..))
            } else {
                Box::new(entries_list.iter())
            };

        for (_id, entry) in &mut iterator {
            response.push(entry.clone());
            entry_counter += 1;

            if entry_counter >= page_size {
                break;
            }
        }

        let page_token = iterator.next().map(|x| x.0.clone());

        Ok((response, page_token))
    }
}

#[cfg(test)]
mod tests {

    use core_objects::{
        AttestationConfig, EntryNodeAttestation, NodeAttestationPlugin, NodeSelector, SPIFFEID,
    };
    use matches::assert_matches;

    use super::*;

    fn init_entry_test() -> (Catalog, RegistrationEntry, RegistrationEntry) {
        let spiffe_id = SPIFFEID {
            trust_domain: "trust_domain".to_string(),
            path: "path".to_string(),
        };

        let entry1 = RegistrationEntry {
            id: String::from("id"),
            other_identities: Vec::new(),
            spiffe_id,
            attestation_config: AttestationConfig::Node(EntryNodeAttestation {
                value: vec![
                    NodeSelector::Cluster("selector1".to_string()),
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

        let mut entry2 = entry1.clone();
        entry2.id = "id2".to_string();
        let catalog = Catalog::new();

        (catalog, entry1, entry2)
    }

    #[tokio::test]
    async fn get_entry_error_path() {
        let (catalog, _entry1, _entry2) = init_entry_test();
        let error = catalog.get_entry("dummy").await.unwrap_err();
        let error = *error.downcast::<Error>().unwrap();

        assert_matches!(error, Error::EntryNotFound(_));
    }

    #[tokio::test]
    async fn get_entry_happy_path() {
        let (catalog, entry1, entry2) = init_entry_test();
        let entries = vec![entry1.clone(), entry2];
        catalog.batch_create(entries).await.unwrap();

        catalog.get_entry(&entry1.id).await.unwrap();
    }

    #[tokio::test]
    async fn create_registration_entry_test_happy_path() {
        let (catalog, entry1, entry2) = init_entry_test();
        let entries = vec![entry1, entry2];
        catalog.batch_create(entries).await.unwrap();
    }

    #[tokio::test]
    async fn create_registration_entry_test_duplicate_entry() {
        let (catalog, entry1, entry2) = init_entry_test();
        let entries = vec![entry1.clone(), entry2.clone()];

        catalog.batch_create(entries.clone()).await.unwrap();

        let results = catalog.batch_create(entries).await.unwrap_err();

        for (_id, result) in results {
            let result = *result.downcast::<Error>().unwrap();

            assert_matches!(result, Error::DuplicatedEntry(_));
        }
    }

    #[tokio::test]
    async fn update_registration_entry_test_happy_path() {
        let (catalog, entry1, entry2) = init_entry_test();
        let entries = vec![entry1, entry2];

        catalog.batch_create(entries.clone()).await.unwrap();

        catalog.batch_update(entries).await.unwrap();
    }

    #[tokio::test]
    async fn update_registration_entry_test_entry_not_exist() {
        let (catalog, entry1, entry2) = init_entry_test();
        let entries = vec![entry1, entry2];

        let results = catalog.batch_update(entries).await.unwrap_err();
        for (_id, result) in results {
            let result = *result.downcast::<Error>().unwrap();

            assert_matches!(result, Error::EntryNotFound(_));
        }
    }

    #[tokio::test]
    async fn delete_registration_entry_test_happy_path() {
        let (catalog, entry1, entry2) = init_entry_test();
        let ids = vec![entry1.id.clone(), entry2.id.clone()];
        let entries = vec![entry1, entry2];

        catalog.batch_create(entries.clone()).await.unwrap();

        catalog.batch_delete(&ids).await.unwrap();
    }

    #[tokio::test]
    async fn delete_registration_entry_test_entry_not_exist() {
        let (catalog, entry1, entry2) = init_entry_test();
        let ids = vec![entry1.id.clone(), entry2.id.clone()];

        let results = catalog.batch_delete(&ids).await.unwrap_err();
        for (_id, result) in results {
            let result = *result.downcast::<Error>().unwrap();

            assert_matches!(result, Error::EntryNotFound(_));
        }
    }

    #[tokio::test]
    async fn get_registration_entry_test_happy_path() {
        let (catalog, entry1, entry2) = init_entry_test();
        let ids = vec![entry1.id.clone(), entry2.id.clone()];
        let entries = vec![entry1, entry2];

        catalog.batch_create(entries).await.unwrap();

        let results = catalog.batch_get(&ids).await;
        for (_id, result) in results {
            result.unwrap();
        }
    }

    #[tokio::test]
    async fn get_registration_entry_test_entry_not_exist() {
        let (catalog, entry1, entry2) = init_entry_test();
        let ids = vec![entry1.id.clone(), entry2.id.clone()];

        let results = catalog.batch_get(&ids).await;
        for (_id, result) in results {
            let result = *result.unwrap_err().downcast::<Error>().unwrap();

            assert_matches!(result, Error::EntryNotFound(_));
        }
    }
}
