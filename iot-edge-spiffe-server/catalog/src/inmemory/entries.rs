// Copyright (c) Microsoft. All rights reserved.

use server_admin_api::RegistrationEntry;

use crate::Entries;

use super::{error::Error, Catalog};

#[async_trait::async_trait]
impl Entries for Catalog {
    type Error = crate::inmemory::Error;

    async fn batch_create(
        &self,
        entries: Vec<RegistrationEntry>,
    ) -> Vec<(String, Result<(), Self::Error>)> {
        let mut entries_list = self.entries_list.lock();
        let mut results = Vec::new();

        for entry in entries {
            let result = if entries_list.contains_key(&entry.id) {
                (entry.id.clone(), Err(Error::DuplicatedEntry(entry.id)))
            } else {
                let id = entry.id.clone();
                entries_list.insert(entry.id.clone(), entry);
                (id, Ok(()))
            };

            results.push(result);
        }

        results
    }

    async fn batch_update(
        &self,
        entries: Vec<RegistrationEntry>,
    ) -> Vec<(String, Result<(), Self::Error>)> {
        let mut entries_list = self.entries_list.lock();
        let mut results = Vec::new();

        for entry in entries {
            let result = if let Some(entry_ptr) = entries_list.get_mut(&entry.id) {
                let id = entry.id.clone();
                *entry_ptr = entry;
                (id, Ok(()))
            } else {
                (
                    entry.id.clone(),
                    Err(Error::EntryNotFound(entry.id.clone())),
                )
            };

            results.push(result);
        }

        results
    }

    async fn batch_delete(&self, ids: &[String]) -> Vec<(String, Result<(), Self::Error>)> {
        let mut entries_list = self.entries_list.lock();
        let mut results = Vec::new();

        for id in ids {
            let result = if entries_list.remove(id).is_some() {
                (id.clone(), Ok(()))
            } else {
                (id.clone(), Err(Error::EntryNotFound(id.to_string())))
            };

            results.push(result);
        }

        results
    }

    async fn batch_get(
        &self,
        ids: &[String],
    ) -> Vec<(String, Result<RegistrationEntry, Self::Error>)> {
        let entries_list = self.entries_list.lock();
        let mut results = Vec::new();

        for id in ids {
            let entry = entries_list.get(id);

            let result = if let Some(entry) = entry {
                (id.clone(), Ok(entry.clone()))
            } else {
                (id.clone(), Err(Error::EntryNotFound(id.to_string())))
            };

            results.push(result);
        }

        results
    }

    async fn list_all(
        &self,
        page_token: Option<String>,
        page_size: usize,
    ) -> Result<(Vec<RegistrationEntry>, Option<String>), Self::Error> {
        let entries_list = self.entries_list.lock();

        let mut response: Vec<RegistrationEntry> = Vec::new();
        let mut entry_counter = 0;

        if page_size == 0 {
            return Err(Error::InvalidPageSize());
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
    use server_admin_api::RegistrationEntry;

    use matches::assert_matches;

    use super::*;

    fn init_entry_test() -> (Catalog, RegistrationEntry, RegistrationEntry) {
        let entry1 = RegistrationEntry {
            id: String::from("id"),
            iot_hub_id: None,
            spiffe_id: String::from("spiffe id"),
            parent_id: None,
            selectors: [String::from("selector1"), String::from("selector2")].to_vec(),
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
    async fn create_registration_entry_test_happy_path() {
        let (catalog, entry1, entry2) = init_entry_test();
        let entries = [entry1, entry2].to_vec();
        let results = catalog.batch_create(entries).await;

        for (_id, result) in results {
            result.unwrap();
        }
    }

    #[tokio::test]
    async fn create_registration_entry_test_duplicate_entry() {
        let (catalog, entry1, entry2) = init_entry_test();
        let entries = [entry1.clone(), entry2.clone()].to_vec();

        let results = catalog.batch_create(entries.clone()).await;
        for (_id, result) in results {
            result.unwrap();
        }

        let results = catalog.batch_create(entries).await;
        for (_id, result) in results {
            assert_matches!(result.unwrap_err(), Error::DuplicatedEntry(_));
        }
    }

    #[tokio::test]
    async fn update_registration_entry_test_happy_path() {
        let (catalog, entry1, entry2) = init_entry_test();
        let entries = [entry1, entry2].to_vec();

        let results = catalog.batch_create(entries.clone()).await;
        for (_id, result) in results {
            result.unwrap();
        }

        let results = catalog.batch_update(entries).await;
        for (_id, result) in results {
            result.unwrap();
        }
    }

    #[tokio::test]
    async fn update_registration_entry_test_entry_not_exist() {
        let (catalog, entry1, entry2) = init_entry_test();
        let entries = [entry1, entry2].to_vec();

        let results = catalog.batch_update(entries).await;
        for (_id, result) in results {
            assert_matches!(result.unwrap_err(), Error::EntryNotFound(_));
        }
    }

    #[tokio::test]
    async fn delete_registration_entry_test_happy_path() {
        let (catalog, entry1, entry2) = init_entry_test();
        let ids = [entry1.id.clone(), entry2.id.clone()].to_vec();
        let entries = [entry1, entry2].to_vec();

        let results = catalog.batch_create(entries.clone()).await;
        for (_id, result) in results {
            result.unwrap();
        }

        let results = catalog.batch_delete(&ids).await;
        for (_id, result) in results {
            result.unwrap();
        }
    }

    #[tokio::test]
    async fn delete_registration_entry_test_entry_not_exist() {
        let (catalog, entry1, entry2) = init_entry_test();
        let ids = [entry1.id.clone(), entry2.id.clone()].to_vec();

        let results = catalog.batch_delete(&ids).await;
        for (_id, result) in results {
            assert_matches!(result.unwrap_err(), Error::EntryNotFound(_));
        }
    }

    #[tokio::test]
    async fn get_registration_entry_test_happy_path() {
        let (catalog, entry1, entry2) = init_entry_test();
        let ids = [entry1.id.clone(), entry2.id.clone()].to_vec();
        let entries = [entry1, entry2].to_vec();

        let results = catalog.batch_create(entries).await;
        for (_id, result) in results {
            result.unwrap();
        }

        let results = catalog.batch_get(&ids).await;
        for (_id, result) in results {
            result.unwrap();
        }
    }

    #[tokio::test]
    async fn get_registration_entry_test_entry_not_exist() {
        let (catalog, entry1, entry2) = init_entry_test();
        let ids = [entry1.id.clone(), entry2.id.clone()].to_vec();

        let results = catalog.batch_get(&ids).await;
        for (_id, result) in results {
            assert_matches!(result.unwrap_err(), Error::EntryNotFound(_));
        }
    }
}
