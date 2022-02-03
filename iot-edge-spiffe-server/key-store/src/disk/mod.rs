use std::{fs, path::{Path, PathBuf}};

use config::KeyPluginConfigDisk;
use openssl::{
    ec, nid,
    pkey::{self, PKey, Public},
    rsa,
};

pub mod error;

use error::Error;

use crate::{KeyPlugin, KeyType};

struct KeyPair {
    public_key : pkey::PKey<pkey::Public>,
    private_key :PKey<pkey::Private>,
}

pub struct Plugin {
    key_base_path: PathBuf,
}


impl Plugin {
    #[must_use]
    pub fn new(config : &KeyPluginConfigDisk) -> Self {
        let key_base_path = Path::new(&config.key_base_path).to_path_buf();
        Plugin {
            key_base_path,
        }
    }

    fn get_key_path(&self, id: &str) -> PathBuf {
        let mut path = self.key_base_path.clone();
        let key_name = Path::new(id);
        path.push(key_name); 
        
        path
    }
}

#[async_trait::async_trait]
impl KeyPlugin for Plugin {
    type Error = crate::disk::Error;

    async fn create_key_pair_if_not_exists(
        &self,
        id: &str,
        key_type: KeyType,
    ) -> Result<(), Error> {
        let path = &self.get_key_path(id);

        if load_inner(path)?.is_none() {
            create_inner(path, key_type)?;
            if load_inner(path)?.is_none() {
                return Err(Error::KeyNotFound(
                    "key created successfully but could not be found".to_string(),
                ));
            }
        }
    
        Ok(())
    }
    
    async fn sign(
        &self,
        id: &str,
        key_type: KeyType,
        digest: &[u8],
    ) -> Result<(usize, Vec<u8>), Error> {
        let path = &self.get_key_path(id);

        let key_pair = load_inner(path)?
            .ok_or_else(|| Error::KeyNotFound("Could not find key for signing".to_string()))?;
    
        let private_key = key_pair.private_key;
    
        let result = match (key_type, private_key.ec_key(), private_key.rsa()) {
            (KeyType::ECP256, Ok(ec_key), _) => {
                let signature_len = {
                    let ec_key = foreign_types_shared::ForeignType::as_ptr(&ec_key);
                    unsafe {
                        let signature_len = openssl_sys2::ECDSA_size(ec_key);
                        std::convert::TryInto::try_into(signature_len).map_err(|err| {
                            Error::ConvertToUsize(err, "ECDSA_size returned invalid value".to_string())
                        })
                    }
                }?;
    
                let signature = openssl::ecdsa::EcdsaSig::sign(digest, &ec_key)?;
                let signature = signature.to_der()?;
    
                Some((signature_len, signature))
            }
    
            _ => None,
        };
    
        result.ok_or_else(Error::UnsupportedMechanismType)
    }
    
    async fn get_public_key(&self, id: &str) -> Result<PKey<Public>, Error> {
        let path = &self.get_key_path(id);

        let key_pair =
            load_inner(path)?.ok_or_else(|| Error::KeyNotFound("Cannot get public key".to_string()))?;
    
        Ok(key_pair.public_key)
    }
    
    async fn delete_key_pair(&self, id: &str) -> Result<(), Error> {
        let path = &self.get_key_path(id);

        fs::remove_file(path).map_err(Error::FileDelete)
    }

    
}
    
fn load_inner(path: &Path) -> Result<Option<KeyPair>, Error> {
    match fs::read(path) {
        Ok(private_key_pem) => {
            let private_key = openssl::pkey::PKey::private_key_from_pem(&private_key_pem)?;

            // Copy private_key's public parameters into a new public key
            let public_key_der = private_key.public_key_to_der()?;
            let public_key = openssl::pkey::PKey::public_key_from_der(&public_key_der)?;

            Ok(Some(KeyPair{public_key, private_key}))
        }

        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),

        Err(err) => Err(Error::FileReadError(err)),
    }
}

fn create_inner(path: &Path, preferred_algorithm: KeyType) -> Result<(), Error> {
    let private_key = match preferred_algorithm {
        KeyType::ECP256 => {
            let mut group = ec::EcGroup::from_curve_name(nid::Nid::X9_62_PRIME256V1)?;
            group.set_asn1_flag(ec::Asn1Flag::NAMED_CURVE);
            let ec_key = ec::EcKey::generate(&group)?;
            pkey::PKey::from_ec_key(ec_key)?
        }

        KeyType::RSA2048 => {
            let rsa = rsa::Rsa::generate(2048)?;
            pkey::PKey::from_rsa(rsa)?
        }

        KeyType::RSA4096 => {
            let rsa = rsa::Rsa::generate(4096)?;
            pkey::PKey::from_rsa(rsa)?
        }
    };

    let private_key_pem = private_key.private_key_to_pem_pkcs8()?;
    fs::write(path, &private_key_pem).map_err(Error::FileWrite)
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::*;

    fn init() -> Plugin {
        let config = KeyPluginConfigDisk { key_base_path: ".".to_string() };
        let plugin = Plugin::new(&config);

        plugin
    }    

    #[tokio::test]
    async fn create_key_pair_happy_path_tests() {
        let plugin = init();

        let id = Uuid::new_v4().to_string();

        let file = format!("./{}", id);

        plugin.create_key_pair_if_not_exists(&id, KeyType::ECP256).await.unwrap();

        // Check file is present
        let metadata = fs::metadata(file.clone()).unwrap();

        plugin.create_key_pair_if_not_exists(&id, KeyType::ECP256).await.unwrap();
        let metadata2 = fs::metadata(file.clone()).unwrap(); 

        // Check file has not been overwritten
        assert_eq!(metadata.modified().unwrap(), metadata2.modified().unwrap());

        // Clean up
        fs::remove_file(file).unwrap();
    }

    #[tokio::test]
    async fn delete_key_pair_happy_path_tests() {
        let plugin = init();

        let id = Uuid::new_v4().to_string();

        let file = format!("./{}", id);

        plugin.create_key_pair_if_not_exists(&id, KeyType::ECP256).await.unwrap();

        plugin.delete_key_pair(&id).await.unwrap();

        // Clean up and verify
        let error = fs::remove_file(file).unwrap_err();

        assert_eq!(std::io::ErrorKind::NotFound, error.kind());
    }

    #[tokio::test]
    async fn delete_key_pair_error_path_tests() {
        let plugin = init();

        let id = Uuid::new_v4().to_string();

        let error = plugin.delete_key_pair(&id).await.unwrap_err();

        if let Error::FileDelete(_) = error {
        } else {
            panic!("Wrong error type returned for delete_key_pair")
        };
    }

    #[tokio::test]
    async fn get_public_key_happy_path() {
        let plugin = init();

        let id = Uuid::new_v4().to_string();

        let file = format!("./{}", id);

        plugin.create_key_pair_if_not_exists(&id, KeyType::ECP256).await.unwrap();

        let _pub_key = plugin.get_public_key(&id).await.unwrap();

        fs::remove_file(file).unwrap();
    }    

    #[tokio::test]
    async fn get_public_key_error_path() {
        let plugin = init();

        let id = Uuid::new_v4().to_string();

        let error = plugin.get_public_key(&id).await.unwrap_err();

        if let Error::KeyNotFound(_) = error {
        } else {
            panic!("Wrong error type returned for get_public_key")
        };
    }  
    
    #[tokio::test]
    async fn get_sign_happy_path() {
        let plugin = init();

        let id = Uuid::new_v4().to_string();

        let file = format!("./{}", id);

        plugin.create_key_pair_if_not_exists(&id, KeyType::ECP256).await.unwrap();

        let digest = "hello world".as_bytes();

        let _signature= plugin.sign(&id,  KeyType::ECP256, digest).await.unwrap();

        fs::remove_file(file).unwrap();        
    }     

    #[tokio::test]
    async fn get_sign_error_path() {
        let plugin = init();

        let id = Uuid::new_v4().to_string();

        let digest = "hello world".as_bytes();

        let error= plugin.sign(&id,  KeyType::ECP256, digest).await.unwrap_err();

        if let Error::KeyNotFound(_) = error {
        } else {
            panic!("Wrong error type returned for get_public_key")
        };       
    }        
}

