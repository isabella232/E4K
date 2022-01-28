// Copyright (c) Microsoft. All rights reserved.

use std::{collections::BTreeMap, sync::Arc};

use common_admin_api::RegistrationEntry;
use futures_util::lock::Mutex;

pub struct InMemoryCatalog {
    entries_list: Arc<Mutex<BTreeMap<String, RegistrationEntry>>>,
}

impl InMemoryCatalog {
    pub fn new() -> Self {
        InMemoryCatalog {
            entries_list: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }
}

#[async_trait::async_trait]
impl crate::Catalog for InMemoryCatalog {
    async fn create_registration_entry(
        &self,
        entry: RegistrationEntry,
    ) -> Result<(), crate::Error> {
        let mut entries_list = self.entries_list.lock().await;

        if entries_list.contains_key(&entry.id) {
            return Err(crate::Error::DuplicatedEntry(format!(
                "Entry {} already exist",
                entry.id
            )));
        }

        entries_list.insert(entry.id.clone(), entry);

        Ok(())
    }

    async fn update_registration_entry(
        &self,
        entry: RegistrationEntry,
    ) -> Result<(), crate::Error> {
        let mut entries_list = self.entries_list.lock().await;

        if entries_list.contains_key(&entry.id) {
            entries_list.insert(entry.id.clone(), entry);
        } else {
            return Err(crate::Error::EntryDoNotExist(format!(
                "Cannot update entry {}, it does not exist",
                entry.id
            )));
        }

        Ok(())
    }

    async fn get_registration_entry(&self, id: &str) -> Result<RegistrationEntry, crate::Error> {
        let entries_list = self.entries_list.lock().await;

        let entry = entries_list.get(id);

        if let Some(entry) = entry {
            Ok(entry.clone())
        } else {
            Err(crate::Error::EntryDoNotExist(format!(
                "Entry {} do not exist",
                id
            )))
        }
    }

    async fn list_registration_entries(
        &self,
        page_token: Option<String>,
        page_size: usize,
    ) -> Result<(Vec<RegistrationEntry>, Option<String>), crate::Error> {
        let entries_list = self.entries_list.lock().await;

        let mut response: Vec<RegistrationEntry> = Vec::new();
        let mut entry_counter = 0;

        if page_size == 0 {
            return Err(crate::Error::InvalidArguments(
                "Invalid page size".to_string(),
            ));
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

    async fn delete_registration_entry(&self, id: &str) -> Result<(), crate::Error> {
        let mut entries_list = self.entries_list.lock().await;

        if entries_list.contains_key(id) {
            entries_list.remove(id);
        } else {
            return Err(crate::Error::EntryDoNotExist(format!(
                "Entry {} do not exist",
                id
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use common_admin_api::RegistrationEntry;

    use crate::{error::Error, Catalog};

    use super::*;

    fn init() -> (Box<dyn Catalog>, RegistrationEntry) {
        let entry = RegistrationEntry {
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
        let catalog = InMemoryCatalog::new();

        (Box::new(catalog), entry)
    }

    #[tokio::test]
    async fn create_registration_entry_test_happy_path() {
        let (catalog, entry) = init();

        let res = catalog.create_registration_entry(entry).await;

        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn create_registration_entry_test_duplicate_entry() {
        let (catalog, entry) = init();

        let _res = catalog.create_registration_entry(entry.clone()).await;
        let res = catalog.create_registration_entry(entry).await.unwrap_err();

        if let Error::DuplicatedEntry(_) = res {
        } else {
            panic!("Wrong error type returned for create_registration_entry")
        };
    }

    #[tokio::test]
    async fn update_registration_entry_test_happy_path() {
        let (catalog, entry) = init();

        let _res = catalog.create_registration_entry(entry.clone()).await;
        let res = catalog.update_registration_entry(entry).await;

        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn update_registration_entry_test_entry_not_exist() {
        let (catalog, entry) = init();

        let res = catalog.update_registration_entry(entry).await.unwrap_err();

        if let Error::EntryDoNotExist(_) = res {
        } else {
            panic!("Wrong error type returned for create_registration_entry")
        };
    }

    #[tokio::test]
    async fn get_registration_entry_test_happy_path() {
        let (catalog, entry) = init();

        let _res = catalog.create_registration_entry(entry.clone()).await;
        let res = catalog.get_registration_entry(&entry.id).await;

        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn get_registration_entry_test_entry_not_exist() {
        let (catalog, entry) = init();

        let res = catalog.get_registration_entry(&entry.id).await.unwrap_err();

        if let Error::EntryDoNotExist(_) = res {
        } else {
            panic!("Wrong error type returned for create_registration_entry")
        };
    }
}
