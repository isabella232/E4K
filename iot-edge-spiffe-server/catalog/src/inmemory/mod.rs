// Copyright (c) Microsoft. All rights reserved.
mod error;

use std::{collections::{BTreeMap, HashMap}, sync::Arc};

use futures_util::lock::Mutex;
use openssl::pkey::{PKey, Public};
use server_admin_api::RegistrationEntry;

use self::error::Error;

pub struct InMemoryCatalog {
    entries_list: Arc<Mutex<BTreeMap<String, RegistrationEntry>>>,
    // Since this is in memory implementation, there is only one trust domain
    trust_domain_store: Arc<Mutex<HashMap<String, PKey<Public>>>>,
}

impl InMemoryCatalog {
    #[must_use]
    pub fn new() -> Self {
        InMemoryCatalog {
            entries_list: Arc::new(Mutex::new(BTreeMap::new())),
            trust_domain_store: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl Default for InMemoryCatalog {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl crate::Catalog for InMemoryCatalog {
    type Error = crate::inmemory::Error;

    async fn create_registration_entry(&self, entry: RegistrationEntry) -> Result<(), Self::Error> {
        let mut entries_list = self.entries_list.lock().await;

        if entries_list.contains_key(&entry.id) {
            return Err(Error::DuplicatedEntry(entry.id));
        }

        entries_list.insert(entry.id.clone(), entry);

        Ok(())
    }

    async fn update_registration_entry(&self, entry: RegistrationEntry) -> Result<(), Self::Error> {
        let mut entries_list = self.entries_list.lock().await;

        let entry_ptr = entries_list
            .get_mut(&entry.id)
            .ok_or_else(|| Error::EntryNotFound(entry.id.clone()))?;

        *entry_ptr = entry;

        Ok(())
    }

    async fn get_registration_entry(&self, id: &str) -> Result<RegistrationEntry, Self::Error> {
        let entries_list = self.entries_list.lock().await;

        let entry = entries_list.get(id);

        if let Some(entry) = entry {
            Ok(entry.clone())
        } else {
            Err(Error::EntryNotFound(id.to_string()))
        }
    }

    async fn list_registration_entries(
        &self,
        page_token: Option<String>,
        page_size: usize,
    ) -> Result<(Vec<RegistrationEntry>, Option<String>), Self::Error> {
        let entries_list = self.entries_list.lock().await;

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

    async fn delete_registration_entry(&self, id: &str) -> Result<(), Self::Error> {
        let mut entries_list = self.entries_list.lock().await;

        if entries_list.contains_key(id) {
            entries_list.remove(id);
        } else {
            return Err(Error::EntryNotFound(id.to_string()));
        }

        Ok(())
    }

    async fn add_key_to_trust_domain_store(&self, _trust_domain: &str, kid: &str, public_key: PKey<Public>) -> Result<(), Self::Error> {
        let mut trust_domain_store = self.trust_domain_store.lock().await;

        if trust_domain_store.contains_key(kid) {
            return Err(Error::DuplicatedKey(kid.to_string()));
        }

        trust_domain_store.insert(kid.to_string(), public_key);

        Ok(())
    }

    async fn remove_key_trust_domain_store(&self, _trust_domain: &str, kid: &str) -> Result<(), Self::Error> {
        let mut trust_domain_store = self.trust_domain_store.lock().await;

        if trust_domain_store.contains_key(kid) {
            trust_domain_store.remove(kid);
        } else {
            return Err(Error::KeyNotFound(kid.to_string()));
        }

        Ok(())
    }

    async fn get_keys_from_trust_domain_store(&self, _trust_domain: &str) -> Result<Vec<PKey<Public>>, Self::Error> {
        let trust_domain_store = self.trust_domain_store.lock().await;

        Ok(trust_domain_store.values().cloned().collect::<Vec<PKey<Public>>>())
    } 
}

#[cfg(test)]
mod tests {
    use server_admin_api::RegistrationEntry;

    use crate::Catalog;

    use super::*;

    fn init_entry_test() -> (InMemoryCatalog, RegistrationEntry) {
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

        (catalog, entry)
    }

    #[tokio::test]
    async fn create_registration_entry_test_happy_path() {
        let (catalog, entry) = init_entry_test();

        catalog.create_registration_entry(entry).await.unwrap();
    }

    #[tokio::test]
    async fn create_registration_entry_test_duplicate_entry() {
        let (catalog, entry) = init_entry_test();

        catalog.create_registration_entry(entry.clone()).await.unwrap();
        let res = catalog.create_registration_entry(entry).await.unwrap_err();
        if let Error::DuplicatedEntry(_) = res {
        } else {
            panic!("Wrong error type returned for create_registration_entry")
        };
    }

    #[tokio::test]
    async fn update_registration_entry_test_happy_path() {
        let (catalog, entry) = init_entry_test();

        catalog.create_registration_entry(entry.clone()).await.unwrap();
        catalog.update_registration_entry(entry).await.unwrap();
    }

    #[tokio::test]
    async fn update_registration_entry_test_entry_not_exist() {
        let (catalog, entry) = init_entry_test();

        let res = catalog.update_registration_entry(entry).await.unwrap_err();
        if let Error::EntryNotFound(_) = res {
        } else {
            panic!("Wrong error type returned for update_registration_entry")
        };
    }

    #[tokio::test]
    async fn delete_registration_entry_test_happy_path() {
        let (catalog, entry) = init_entry_test();

        catalog.create_registration_entry(entry.clone()).await.unwrap();
        catalog.delete_registration_entry(&entry.id).await.unwrap();
    }   
    
    #[tokio::test]
    async fn delete_registration_entry_test_entry_not_exist() {
        let (catalog, entry) = init_entry_test();

        let res = catalog.delete_registration_entry(&entry.id).await.unwrap_err();
        if let Error::EntryNotFound(_) = res {
        } else {
            panic!("Wrong error type returned for delete_registration_entry")
        };
    }    

    #[tokio::test]
    async fn get_registration_entry_test_happy_path() {
        let (catalog, entry) = init_entry_test();

        let _res = catalog.create_registration_entry(entry.clone()).await;
        let res = catalog.get_registration_entry(&entry.id).await;

        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn get_registration_entry_test_entry_not_exist() {
        let (catalog, entry) = init_entry_test();

        let res = catalog.get_registration_entry(&entry.id).await.unwrap_err();
        if let Error::EntryNotFound(_) = res {
        } else {
            panic!("Wrong error type returned for get_registration_entry")
        };
    }

    fn init_key_test() -> (InMemoryCatalog, PKey<Public>) {
        let public_key_der= PKey::generate_ed25519().unwrap().public_key_to_der().unwrap();
        let public_key = openssl::pkey::PKey::public_key_from_der(&public_key_der).unwrap();
        
        let catalog = InMemoryCatalog::new();

        (catalog, public_key)
    }

    #[tokio::test]
    async fn add_key_to_trust_domain_store_test_happy_path() {
        let (catalog, public_key) = init_key_test();

        catalog.add_key_to_trust_domain_store("dummy", "my_key", public_key).await.unwrap();
    }

    #[tokio::test]
    async fn add_key_to_trust_domain_store_test_duplicate_entry() {
        let (catalog, public_key) = init_key_test();

        let _res = catalog.add_key_to_trust_domain_store("dummy", "my_key", public_key.clone()).await.unwrap();
        let res = catalog.add_key_to_trust_domain_store("dummy", "my_key", public_key).await.unwrap_err();
        if let Error::DuplicatedKey(_) = res {
        } else {
            panic!("Wrong error type returned for add_key_to_trust_domain_store")
        };
    }

    #[tokio::test]
    async fn remove_key_trust_domain_store_test_happy_path() {
        let (catalog, public_key) = init_key_test();

        catalog.add_key_to_trust_domain_store("dummy", "my_key", public_key).await.unwrap();
        catalog.remove_key_trust_domain_store("dummy", "my_key").await.unwrap();
    }   
    
    #[tokio::test]
    async fn remove_key_trust_domain_store_test_entry_not_exist() {
        let (catalog, public_key) = init_key_test();

        catalog.add_key_to_trust_domain_store("dummy", "my_key", public_key).await.unwrap();
        let res = catalog.remove_key_trust_domain_store("dummy", "another_key").await.unwrap_err();
        if let Error::KeyNotFound(_) = res {
        } else {
            panic!("Wrong error type returned for remove_key_trust_domain_store")
        };
    } 

    #[tokio::test]
    async fn get_keys_from_trust_domain_store_test_happy_path() {
        let (catalog, public_key) = init_key_test();

        catalog.add_key_to_trust_domain_store("dummy", "my_key", public_key.clone()).await.unwrap();
        catalog.add_key_to_trust_domain_store("dummy", "my_key2", public_key).await.unwrap();

        let keys = catalog.get_keys_from_trust_domain_store("dummy").await.unwrap();

        assert_eq!(keys.len(), 2);
    } 
}
