// Copyright (c) Microsoft. All rights reserved.

use std::{path::PathBuf, fs};

use openssl::{nid, ec, pkey, rsa};

use crate::error::Error;

enum KeyPair {
    FileSystem(
        pkey::PKey<pkey::Public>,
        pkey::PKey<pkey::Private>,
    ),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PreferredAlgorithm {
    NistP256,
    Rsa2048,
    Rsa4096,
}

enum KeySignMechanism {
    ECDSA = 0,
}


pub fn create_key_pair_if_not_exists(
    path: &PathBuf,
    preferred_algorithms: &PreferredAlgorithm,
) -> Result<(),Error> {
    if load_inner(path)?.is_none() {
        create_inner(path, preferred_algorithms)?;
        if load_inner(path)?.is_none() {
            return Err(Error::KeyNotFound("key created successfully but could not be found".to_string()));
        }
    }

    Ok(())
}


pub(crate) unsafe fn sign(
    path: &PathBuf,
    mechanism: &KeySignMechanism,
    digest: &[u8],
) -> Result<(usize, Vec<u8>), Error> {
    let key_pair = load_inner(path)?.ok_or_else(|| Error::KeyNotFound("Could not find key for signing".to_string()))?; 

    let private_key = if let KeyPair::FileSystem(_, private_key)  = key_pair{
        private_key
    } else {
        return Err(Error::UnsupportedKeyPairType());
    };

    let result = match (mechanism, private_key.ec_key(), private_key.rsa()) {
        (KeySignMechanism::ECDSA, Ok(ec_key), _) => {
            let signature_len = {
                let ec_key = foreign_types_shared::ForeignType::as_ptr(&ec_key);
                let signature_len = openssl_sys2::ECDSA_size(ec_key);
                let signature_len = std::convert::TryInto::try_into(signature_len)
                    .map_err(|err| {
                        Error::ConvertToUsize(err, "ECDSA_size returned invalid value".to_string())  })?;
                signature_len
            };

            let signature = openssl::ecdsa::EcdsaSig::sign(digest, &ec_key)?;
            let signature = signature.to_der()?;

            Some((signature_len, signature))
        }

        _ => None,
    };

    let result = result.ok_or_else(|| {
        crate::implementation::err_invalid_parameter("mechanism", "unrecognized value")
    })?;

    Ok(result)
}

pub fn delete_key_pair(
    path: &PathBuf,
) -> Result<(),Error> {
    fs::remove_file(path).map_err(|op| Error::FileDelete(op))
}



fn load_inner(
    path: &PathBuf,
) -> Result<Option<KeyPair>, Error> {
    match fs::read(path) {
        Ok(private_key_pem) => {
            let private_key = openssl::pkey::PKey::private_key_from_pem(&private_key_pem)?;

            // Copy private_key's public parameters into a new public key
            let public_key_der = private_key.public_key_to_der()?;
            let public_key = openssl::pkey::PKey::public_key_from_der(&public_key_der)?;

            return Ok(Some(KeyPair::FileSystem(public_key, private_key)));
        }

        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),

        Err(err) => return Err(Error::FileReadError(err)),
    }
}


fn create_inner(
    path: &PathBuf,
    preferred_algorithm: &PreferredAlgorithm,
) -> Result<(), Error> {

    let private_key = match preferred_algorithm {
        PreferredAlgorithm::NistP256 => {
            let mut group =
                ec::EcGroup::from_curve_name(nid::Nid::X9_62_PRIME256V1)?;
            group.set_asn1_flag(ec::Asn1Flag::NAMED_CURVE);
            let ec_key = ec::EcKey::generate(&group)?;
            let private_key = pkey::PKey::from_ec_key(ec_key)?;
            private_key
        }

        PreferredAlgorithm::Rsa2048 => {
            let rsa = rsa::Rsa::generate(2048)?;
            let private_key = pkey::PKey::from_rsa(rsa)?;
            private_key
        }

        PreferredAlgorithm::Rsa4096 => {
            let rsa = rsa::Rsa::generate(4096)?;
            let private_key = pkey::PKey::from_rsa(rsa)?;
            private_key
        }
    };

    let private_key_pem = private_key.private_key_to_pem_pkcs8()?;
    fs::write(path, &private_key_pem).map_err(|op| Error::FileWrite(op))?;

    Ok(())
}


