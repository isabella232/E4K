// Copyright (c) Microsoft. All rights reserved.

use core_objects::JWK;

use crate::TrustBundleStore;

use super::{error::Error, Catalog};

#[async_trait::async_trait]
impl TrustBundleStore for Catalog {
    async fn add_jwk(
        &self,
        _trust_domain: &str,
        jwk: JWK,
    ) -> Result<(), Box<dyn std::error::Error + Send>> {
        let mut jwt_trust_domain = self.jwt_trust_domain.write();

        if jwt_trust_domain.store.contains_key(&jwk.kid) {
            return Err(Box::new(Error::DuplicatedKey(jwk.kid)));
        }

        jwt_trust_domain.version += 1;
        jwt_trust_domain.store.insert(jwk.kid.clone(), jwk);

        Ok(())
    }

    async fn remove_jwk(
        &self,
        _trust_domain: &str,
        kid: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send>> {
        let mut jwt_trust_domain = self.jwt_trust_domain.write();

        jwt_trust_domain
            .store
            .remove(kid)
            .ok_or_else(|| Box::new(Error::KeyNotFound(kid.to_string())) as _)
            .map(|_| ())?;

        jwt_trust_domain.version += 1;

        Ok(())
    }

    async fn get_jwk(
        &self,
        _trust_domain: &str,
    ) -> Result<(Vec<JWK>, usize), Box<dyn std::error::Error + Send>> {
        let jwt_trust_domain = self.jwt_trust_domain.read();

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
    use core_objects::{Crv, KeyUse, Kty};

    use matches::assert_matches;

    use super::*;

    #[tokio::test]
    async fn add_jwk_test_happy_path() {
        let catalog = Catalog::new();

        let jwk = JWK {
            kid: "my_key".to_string(),
            x: "abc".to_string(),
            y: "abc".to_string(),
            kty: Kty::EC,
            crv: Crv::P256,
            key_use: KeyUse::JWTSVID,
        };

        catalog.add_jwk("dummy", jwk).await.unwrap();
    }

    #[tokio::test]
    async fn add_jwk_test_duplicate_entry() {
        let catalog = Catalog::new();

        let jwk = JWK {
            kid: "my_key".to_string(),
            x: "abc".to_string(),
            y: "abc".to_string(),
            kty: Kty::EC,
            crv: Crv::P256,
            key_use: KeyUse::JWTSVID,
        };

        let _res = catalog.add_jwk("dummy", jwk.clone()).await.unwrap();
        let res = *catalog
            .add_jwk("dummy", jwk)
            .await
            .unwrap_err()
            .downcast::<Error>()
            .unwrap();

        assert_matches!(res, Error::DuplicatedKey(_));
    }

    #[tokio::test]
    async fn remove_jwk_test_happy_path() {
        let catalog = Catalog::new();

        let jwk = JWK {
            kid: "my_key".to_string(),
            x: "abc".to_string(),
            y: "abc".to_string(),
            kty: Kty::EC,
            crv: Crv::P256,
            key_use: KeyUse::JWTSVID,
        };

        catalog.add_jwk("dummy", jwk.clone()).await.unwrap();
        catalog.remove_jwk("dummy", "my_key").await.unwrap();
    }

    #[tokio::test]
    async fn remove_jwk_test_entry_not_exist() {
        let catalog = Catalog::new();

        let jwk = JWK {
            kid: "my_key".to_string(),
            x: "abc".to_string(),
            y: "abc".to_string(),
            kty: Kty::EC,
            crv: Crv::P256,
            key_use: KeyUse::JWTSVID,
        };

        catalog.add_jwk("dummy", jwk).await.unwrap();
        let res = *catalog
            .remove_jwk("dummy", "another_key")
            .await
            .unwrap_err()
            .downcast::<Error>()
            .unwrap();

        assert_matches!(res, Error::KeyNotFound(_));
    }

    #[tokio::test]
    async fn get_keys_from_jwt_trust_domain_store_test_happy_path() {
        let catalog = Catalog::new();

        let jwk = JWK {
            kid: "my_key".to_string(),
            x: "abc".to_string(),
            y: "abc".to_string(),
            kty: Kty::EC,
            crv: Crv::P256,
            key_use: KeyUse::JWTSVID,
        };
        catalog.add_jwk("dummy", jwk.clone()).await.unwrap();

        let jwk = JWK {
            kid: "my_key2".to_string(),
            x: "abc".to_string(),
            y: "abc".to_string(),
            kty: Kty::EC,
            crv: Crv::P256,
            key_use: KeyUse::JWTSVID,
        };
        catalog.add_jwk("dummy", jwk).await.unwrap();

        let (keys, version) = catalog.get_jwk("dummy").await.unwrap();

        assert_eq!(keys.len(), 2);
        assert_eq!(version, 2);
    }
}
