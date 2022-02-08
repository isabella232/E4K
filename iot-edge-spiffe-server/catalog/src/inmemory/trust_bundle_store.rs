// Copyright (c) Microsoft. All rights reserved.

use openssl::pkey::{PKey, Public};

use crate::TrustBundleStore;

use super::{error::Error, Catalog};

#[async_trait::async_trait]
impl TrustBundleStore for Catalog {
    type Error = crate::inmemory::Error;

    async fn add_jwt_key(
        &self,
        _trust_domain: &str,
        kid: &str,
        public_key: PKey<Public>,
    ) -> Result<(), Self::Error> {
        let mut jwt_trust_domain_store = self.jwt_trust_domain_store.lock();

        if jwt_trust_domain_store.contains_key(kid) {
            return Err(Error::DuplicatedKey(kid.to_string()));
        }

        jwt_trust_domain_store.insert(kid.to_string(), public_key);

        Ok(())
    }

    async fn remove_jwt_key(&self, _trust_domain: &str, kid: &str) -> Result<(), Self::Error> {
        let mut jwt_trust_domain_store = self.jwt_trust_domain_store.lock();

        if jwt_trust_domain_store.contains_key(kid) {
            jwt_trust_domain_store.remove(kid);
        } else {
            return Err(Error::KeyNotFound(kid.to_string()));
        }

        Ok(())
    }

    async fn get_jwt_keys(&self, _trust_domain: &str) -> Result<Vec<PKey<Public>>, Self::Error> {
        let jwt_trust_domain_store = self.jwt_trust_domain_store.lock();

        Ok(jwt_trust_domain_store
            .values()
            .cloned()
            .collect::<Vec<PKey<Public>>>())
    }
}

#[cfg(test)]
mod tests {
    use openssl::{ec, nid, pkey};

    use matches::assert_matches;

    use super::*;

    fn init_key_test() -> (Catalog, PKey<Public>) {
        let mut group = ec::EcGroup::from_curve_name(nid::Nid::X9_62_PRIME256V1).unwrap();
        group.set_asn1_flag(ec::Asn1Flag::NAMED_CURVE);
        let ec_key = ec::EcKey::generate(&group).unwrap();
        let public_key_der = pkey::PKey::from_ec_key(ec_key)
            .unwrap()
            .public_key_to_der()
            .unwrap();
        let public_key = openssl::pkey::PKey::public_key_from_der(&public_key_der).unwrap();

        let catalog = Catalog::new();

        (catalog, public_key)
    }

    #[tokio::test]
    async fn add_jwt_key_test_happy_path() {
        let (catalog, public_key) = init_key_test();

        catalog
            .add_jwt_key("dummy", "my_key", public_key)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn add_jwt_key_test_duplicate_entry() {
        let (catalog, public_key) = init_key_test();

        let _res = catalog
            .add_jwt_key("dummy", "my_key", public_key.clone())
            .await
            .unwrap();
        let res = catalog
            .add_jwt_key("dummy", "my_key", public_key)
            .await
            .unwrap_err();
        assert_matches!(res, Error::DuplicatedKey(_));
    }

    #[tokio::test]
    async fn remove_jwt_key_test_happy_path() {
        let (catalog, public_key) = init_key_test();

        catalog
            .add_jwt_key("dummy", "my_key", public_key)
            .await
            .unwrap();
        catalog.remove_jwt_key("dummy", "my_key").await.unwrap();
    }

    #[tokio::test]
    async fn remove_jwt_key_test_entry_not_exist() {
        let (catalog, public_key) = init_key_test();

        catalog
            .add_jwt_key("dummy", "my_key", public_key)
            .await
            .unwrap();
        let res = catalog
            .remove_jwt_key("dummy", "another_key")
            .await
            .unwrap_err();
        assert_matches!(res, Error::KeyNotFound(_));
    }

    #[tokio::test]
    async fn get_keys_from_jwt_trust_domain_store_test_happy_path() {
        let (catalog, public_key) = init_key_test();

        catalog
            .add_jwt_key("dummy", "my_key", public_key.clone())
            .await
            .unwrap();
        catalog
            .add_jwt_key("dummy", "my_key2", public_key)
            .await
            .unwrap();

        let keys = catalog.get_jwt_keys("dummy").await.unwrap();

        assert_eq!(keys.len(), 2);
    }
}
