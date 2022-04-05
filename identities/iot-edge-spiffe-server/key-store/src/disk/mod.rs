// Copyright (c) Microsoft. All rights reserved.

use std::path::{Path, PathBuf};

use core_objects::KeyType;
use openssl::{
    ec, nid,
    pkey::{self, PKey, Public},
};
use server_config::KeyStoreConfigDisk;

pub mod error;

use error::Error;
use tokio::fs;

use crate::KeyStore as KeyPluginTrait;

struct KeyPair {
    public_key: pkey::PKey<pkey::Public>,
    private_key: PKey<pkey::Private>,
}

pub struct KeyStore {
    key_base_path: PathBuf,
}

impl KeyStore {
    #[must_use]
    pub fn new(config: &KeyStoreConfigDisk) -> Self {
        let key_base_path = Path::new(&config.key_base_path).to_path_buf();
        KeyStore { key_base_path }
    }

    fn get_key_path(&self, id: &str) -> PathBuf {
        let mut path = self.key_base_path.clone();
        let key_name = Path::new(id);
        path.push(key_name);

        path
    }
}

#[async_trait::async_trait]
impl KeyPluginTrait for KeyStore {
    async fn create_key_pair_if_not_exists(
        &self,
        id: &str,
        key_type: KeyType,
    ) -> Result<PKey<Public>, Box<dyn std::error::Error + Send>> {
        let path = &self.get_key_path(id);

        let key_pair = if let Some(key_pair) = load_inner(path).await? {
            key_pair
        } else {
            create_inner(path, key_type).await?;

            if let Some(key_pair) = load_inner(path).await? {
                key_pair
            } else {
                return Err(Box::new(Error::KeyNotFound(
                    "key created successfully but could not be found".to_string(),
                )));
            }
        };

        Ok(key_pair.public_key)
    }

    async fn sign(
        &self,
        id: &str,
        key_type: KeyType,
        digest: &[u8],
    ) -> Result<(usize, Vec<u8>), Box<dyn std::error::Error + Send>> {
        let path = &self.get_key_path(id);

        let key_pair = load_inner(path).await?.ok_or_else(|| {
            Box::new(Error::KeyNotFound(
                "Could not find key for signing".to_string(),
            )) as _
        })?;

        let private_key = key_pair.private_key;

        match (key_type, private_key.ec_key(), private_key.rsa()) {
            (KeyType::ES256, Ok(ec_key), _) => {
                let signature_len = {
                    let ec_key = foreign_types_shared::ForeignType::as_ptr(&ec_key);
                    unsafe {
                        let signature_len = openssl_sys2::ECDSA_size(ec_key);
                        std::convert::TryInto::try_into(signature_len).map_err(|err| {
                            Box::new(Error::ConvertToUsize(
                                err,
                                "ECDSA_size returned invalid value".to_string(),
                            )) as _
                        })
                    }
                }?;

                let signature = openssl::ecdsa::EcdsaSig::sign(digest, &ec_key)
                    .map_err(|op| Box::new(op) as _)?;
                let signature = signature.to_der().map_err(|op| Box::new(op) as _)?;

                Ok((signature_len, signature))
            }

            _ => Err(Box::new(Error::UnsupportedMechanismType())),
        }
    }

    async fn get_public_key(
        &self,
        id: &str,
    ) -> Result<PKey<Public>, Box<dyn std::error::Error + Send>> {
        let path = &self.get_key_path(id);

        let key_pair = load_inner(path).await?.ok_or_else(|| {
            Box::new(Error::KeyNotFound("Cannot get public key".to_string())) as _
        })?;

        Ok(key_pair.public_key)
    }

    async fn delete_key_pair(&self, id: &str) -> Result<(), Box<dyn std::error::Error + Send>> {
        let path = &self.get_key_path(id);

        fs::remove_file(path)
            .await
            .map_err(|op| Box::new(Error::FileDelete(op)) as _)
    }
}

async fn load_inner(path: &Path) -> Result<Option<KeyPair>, Box<dyn std::error::Error + Send>> {
    match fs::read(path).await {
        Ok(private_key_pem) => {
            let private_key = openssl::pkey::PKey::private_key_from_pem(&private_key_pem)
                .map_err(|op| Box::new(op) as _)?;

            // Copy private_key's public parameters into a new public key
            let public_key_der = private_key
                .public_key_to_der()
                .map_err(|op| Box::new(op) as _)?;
            let public_key = openssl::pkey::PKey::public_key_from_der(&public_key_der)
                .map_err(|op| Box::new(op) as _)?;

            Ok(Some(KeyPair {
                public_key,
                private_key,
            }))
        }

        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),

        Err(err) => Err(Box::new(Error::FileReadError(err))),
    }
}

async fn create_inner(
    path: &Path,
    preferred_algorithm: KeyType,
) -> Result<KeyPair, Box<dyn std::error::Error + Send>> {
    let private_key = match preferred_algorithm {
        KeyType::ES256 => {
            let mut group = ec::EcGroup::from_curve_name(nid::Nid::X9_62_PRIME256V1)
                .map_err(|op| Box::new(op) as _)?;
            group.set_asn1_flag(ec::Asn1Flag::NAMED_CURVE);
            let ec_key = ec::EcKey::generate(&group).map_err(|op| Box::new(op) as _)?;
            pkey::PKey::from_ec_key(ec_key).map_err(|op| Box::new(op) as _)?
        }

        _ => return Err(Box::new(Error::UnimplementedKeyType(preferred_algorithm))),
    };

    let private_key_pem = private_key
        .private_key_to_pem_pkcs8()
        .map_err(|op| Box::new(op) as _)?;
    fs::write(path, &private_key_pem)
        .await
        .map_err(|op| Box::new(Error::FileWrite(op)) as _)?;

    // Copy private_key's public parameters into a new public key
    let public_key_der = private_key
        .public_key_to_der()
        .map_err(|op| Box::new(op) as _)?;
    let public_key = openssl::pkey::PKey::public_key_from_der(&public_key_der)
        .map_err(|op| Box::new(op) as _)?;

    Ok(KeyPair {
        public_key,
        private_key,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use matches::assert_matches;
    use tempdir::TempDir;
    use uuid::Uuid;

    fn init() -> (String, KeyStore) {
        let dir = TempDir::new("test").unwrap();
        let key_base_path = dir.into_path().to_str().unwrap().to_string();
        let config = KeyStoreConfigDisk {
            key_base_path: key_base_path.clone(),
        };
        (key_base_path, KeyStore::new(&config))
    }

    #[tokio::test]
    async fn create_key_pair_happy_path_tests() {
        let (key_base_path, plugin) = init();

        let id = Uuid::new_v4().to_string();

        let file = format!("{}/{}", key_base_path, id);

        plugin
            .create_key_pair_if_not_exists(&id, KeyType::ES256)
            .await
            .unwrap();

        // Check file is present
        let metadata = fs::metadata(file.clone()).await.unwrap();

        plugin
            .create_key_pair_if_not_exists(&id, KeyType::ES256)
            .await
            .unwrap();
        let metadata2 = fs::metadata(file.clone()).await.unwrap();

        // Check file has not been overwritten
        assert_eq!(metadata.modified().unwrap(), metadata2.modified().unwrap());

        // Clean up
        fs::remove_file(file).await.unwrap();
    }

    #[tokio::test]
    async fn delete_key_pair_happy_path_tests() {
        let (key_base_path, plugin) = init();

        let id = Uuid::new_v4().to_string();

        let file = format!("{}/{}", key_base_path, id);

        plugin
            .create_key_pair_if_not_exists(&id, KeyType::ES256)
            .await
            .unwrap();

        plugin.delete_key_pair(&id).await.unwrap();

        // Clean up and verify
        let error = fs::remove_file(file).await.unwrap_err();

        assert_eq!(std::io::ErrorKind::NotFound, error.kind());
    }

    #[tokio::test]
    async fn delete_key_pair_error_path_tests() {
        let (_key_base_path, plugin) = init();

        let id = Uuid::new_v4().to_string();

        let error = *plugin
            .delete_key_pair(&id)
            .await
            .unwrap_err()
            .downcast::<Error>()
            .unwrap();

        assert_matches!(error, Error::FileDelete(_));
    }

    #[tokio::test]
    async fn get_public_key_happy_path() {
        let (key_base_path, plugin) = init();

        let id = Uuid::new_v4().to_string();

        let file = format!("{}/{}", key_base_path, id);

        plugin
            .create_key_pair_if_not_exists(&id, KeyType::ES256)
            .await
            .unwrap();

        let _pub_key = plugin.get_public_key(&id).await.unwrap();

        fs::remove_file(file).await.unwrap();
    }

    #[tokio::test]
    async fn get_public_key_error_path() {
        let (_key_base_path, plugin) = init();

        let id = Uuid::new_v4().to_string();

        let error = *plugin
            .get_public_key(&id)
            .await
            .unwrap_err()
            .downcast::<Error>()
            .unwrap();

        assert_matches!(error, Error::KeyNotFound(_));
    }

    #[tokio::test]
    async fn get_sign_happy_path() {
        let (key_base_path, plugin) = init();

        let id = Uuid::new_v4().to_string();

        let file = format!("{}/{}", key_base_path, id);

        plugin
            .create_key_pair_if_not_exists(&id, KeyType::ES256)
            .await
            .unwrap();

        let digest = "hello world".as_bytes();

        let _signature = plugin.sign(&id, KeyType::ES256, digest).await.unwrap();

        fs::remove_file(file).await.unwrap();
    }

    #[tokio::test]
    async fn get_sign_error_path() {
        let (_key_base_path, plugin) = init();

        let id = Uuid::new_v4().to_string();

        let digest = "hello world".as_bytes();

        let error = *plugin
            .sign(&id, KeyType::ES256, digest)
            .await
            .unwrap_err()
            .downcast::<Error>()
            .unwrap();

        assert_matches!(error, Error::KeyNotFound(_));
    }
}
