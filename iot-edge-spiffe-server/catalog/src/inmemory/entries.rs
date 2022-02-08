// Copyright (c) Microsoft. All rights reserved.

use server_admin_api::RegistrationEntry;

use crate::Entries;

use super::{error::Error, Catalog};

#[async_trait::async_trait]
impl Entries for Catalog {
    type Error = crate::inmemory::Error;

    async fn create_registration_entry(&self, entry: RegistrationEntry) -> Result<(), Self::Error> {
        let mut entries_list = self.entries_list.lock();

        if entries_list.contains_key(&entry.id) {
            return Err(Error::DuplicatedEntry(entry.id));
        }

        entries_list.insert(entry.id.clone(), entry);

        Ok(())
    }

    async fn update_registration_entry(&self, entry: RegistrationEntry) -> Result<(), Self::Error> {
        let mut entries_list = self.entries_list.lock();

        let entry_ptr = entries_list
            .get_mut(&entry.id)
            .ok_or_else(|| Error::EntryNotFound(entry.id.clone()))?;

        *entry_ptr = entry;

        Ok(())
    }

    async fn get_registration_entry(&self, id: &str) -> Result<RegistrationEntry, Self::Error> {
        let entries_list = self.entries_list.lock();

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

    async fn delete_registration_entry(&self, id: &str) -> Result<(), Self::Error> {
        let mut entries_list = self.entries_list.lock();

        if entries_list.contains_key(id) {
            entries_list.remove(id);
        } else {
            return Err(Error::EntryNotFound(id.to_string()));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use server_admin_api::RegistrationEntry;

    use matches::assert_matches;

    use super::*;

    fn init_entry_test() -> (Catalog, RegistrationEntry) {
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
        let catalog = Catalog::new();

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

        catalog
            .create_registration_entry(entry.clone())
            .await
            .unwrap();
        let res = catalog.create_registration_entry(entry).await.unwrap_err();
        assert_matches!(res, Error::DuplicatedEntry(_));
    }

    #[tokio::test]
    async fn update_registration_entry_test_happy_path() {
        let (catalog, entry) = init_entry_test();

        catalog
            .create_registration_entry(entry.clone())
            .await
            .unwrap();
        catalog.update_registration_entry(entry).await.unwrap();
    }

    #[tokio::test]
    async fn update_registration_entry_test_entry_not_exist() {
        let (catalog, entry) = init_entry_test();

        let res = catalog.update_registration_entry(entry).await.unwrap_err();
        assert_matches!(res, Error::EntryNotFound(_));
    }

    #[tokio::test]
    async fn delete_registration_entry_test_happy_path() {
        let (catalog, entry) = init_entry_test();

        catalog
            .create_registration_entry(entry.clone())
            .await
            .unwrap();
        catalog.delete_registration_entry(&entry.id).await.unwrap();
    }

    #[tokio::test]
    async fn delete_registration_entry_test_entry_not_exist() {
        let (catalog, entry) = init_entry_test();

        let res = catalog
            .delete_registration_entry(&entry.id)
            .await
            .unwrap_err();
        assert_matches!(res, Error::EntryNotFound(_));
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
        assert_matches!(res, Error::EntryNotFound(_));
    }
}
