// Copyright (c) Microsoft. All rights reserved.

use core_objects::JWK;

use crate::TrustBundleStore;

use super::{error::Error, Catalog};

#[async_trait::async_trait]
impl TrustBundleStore for Catalog {
    async fn add_jwt_key(
        &self,
        _trust_domain: &str,
        jwk: JWK,
    ) -> Result<(), Box<dyn std::error::Error + Send>> {
        let mut jwt_trust_domain = self.jwt_trust_domain.lock();

        if jwt_trust_domain.store.contains_key(&jwk.key_id) {
            return Err(Box::new(Error::DuplicatedKey(jwk.key_id)));
        }

        jwt_trust_domain.version += 1;
        jwt_trust_domain.store.insert(jwk.key_id.clone(), jwk);

        Ok(())
    }

    async fn remove_jwt_key(
        &self,
        _trust_domain: &str,
        kid: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send>> {
        let mut jwt_trust_domain = self.jwt_trust_domain.lock();

        jwt_trust_domain
            .store
            .remove(kid)
            .ok_or_else(|| Box::new(Error::KeyNotFound(kid.to_string())) as _)
            .map(|_| ())?;

        jwt_trust_domain.version += 1;

        Ok(())
    }

    async fn get_jwt_keys(
        &self,
        _trust_domain: &str,
    ) -> Result<(Vec<JWK>, usize), Box<dyn std::error::Error + Send>> {
        let jwt_trust_domain = self.jwt_trust_domain.lock();

        Ok((
            jwt_trust_domain
                .store
                .values()
                .cloned()
                .collect::<Vec<JWK>>(),
            jwt_trust_domain.version,
        ))
    }
}

#[cfg(test)]
mod tests {
    use openssl::{ec, nid, pkey};

    use matches::assert_matches;

    use super::*;

    fn init_key_test() -> (Catalog, Vec<u8>) {
        let mut group = ec::EcGroup::from_curve_name(nid::Nid::X9_62_PRIME256V1).unwrap();
        group.set_asn1_flag(ec::Asn1Flag::NAMED_CURVE);
        let ec_key = ec::EcKey::generate(&group).unwrap();
        let public_key = pkey::PKey::from_ec_key(ec_key)
            .unwrap()
            .public_key_to_der()
            .unwrap();

        let catalog = Catalog::new();

        (catalog, public_key)
    }

    #[tokio::test]
    async fn add_jwt_key_test_happy_path() {
        let (catalog, public_key) = init_key_test();

        let jwk = JWK {
            public_key,
            key_id: "my_key".to_string(),
            expiry: 0,
        };

        catalog.add_jwt_key("dummy", jwk).await.unwrap();
    }

    #[tokio::test]
    async fn add_jwt_key_test_duplicate_entry() {
        let (catalog, public_key) = init_key_test();

        let jwk = JWK {
            public_key,
            key_id: "my_key".to_string(),
            expiry: 0,
        };

        let _res = catalog.add_jwt_key("dummy", jwk.clone()).await.unwrap();
        let res = *catalog
            .add_jwt_key("dummy", jwk)
            .await
            .unwrap_err()
            .downcast::<Error>()
            .unwrap();

        assert_matches!(res, Error::DuplicatedKey(_));
    }

    #[tokio::test]
    async fn remove_jwt_key_test_happy_path() {
        let (catalog, public_key) = init_key_test();

        let jwk = JWK {
            public_key,
            key_id: "my_key".to_string(),
            expiry: 0,
        };

        catalog.add_jwt_key("dummy", jwk.clone()).await.unwrap();
        catalog.remove_jwt_key("dummy", "my_key").await.unwrap();
    }

    #[tokio::test]
    async fn remove_jwt_key_test_entry_not_exist() {
        let (catalog, public_key) = init_key_test();

        let jwk = JWK {
            public_key,
            key_id: "my_key".to_string(),
            expiry: 0,
        };

        catalog.add_jwt_key("dummy", jwk).await.unwrap();
        let res = *catalog
            .remove_jwt_key("dummy", "another_key")
            .await
            .unwrap_err()
            .downcast::<Error>()
            .unwrap();

        assert_matches!(res, Error::KeyNotFound(_));
    }

    #[tokio::test]
    async fn get_keys_from_jwt_trust_domain_store_test_happy_path() {
        let (catalog, public_key) = init_key_test();

        let jwk = JWK {
            public_key: public_key.clone(),
            key_id: "my_key".to_string(),
            expiry: 0,
        };

        catalog.add_jwt_key("dummy", jwk.clone()).await.unwrap();

        let jwk = JWK {
            public_key,
            key_id: "my_key2".to_string(),
            expiry: 0,
        };
        catalog.add_jwt_key("dummy", jwk).await.unwrap();

        let (keys, version) = catalog.get_jwt_keys("dummy").await.unwrap();

        assert_eq!(keys.len(), 2);
        assert_eq!(version, 2);
    }
}
