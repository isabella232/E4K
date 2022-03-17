// Copyright (c) Microsoft. All rights reserved.

use crate::error::Error;
use crate::JWTSVIDValidator as JWTSVIDValidatorTrait;
use core_objects::{get_epoch_time, JWTClaims, JWTHeader, JWTType, KeyType, TrustBundle, JWTSVID};
use openssl::{bn::BigNum, nid, sha};

#[derive(Default)]
pub struct JWTSVIDValidator {}

#[async_trait::async_trait]
impl JWTSVIDValidatorTrait for JWTSVIDValidator {
    async fn validate(
        &self,
        jwt_svid_compact: &str,
        trust_bundle: &TrustBundle,
        audience: &str,
    ) -> Result<JWTSVID, Error> {
        let time = get_epoch_time();
        self.validate_inner(jwt_svid_compact, trust_bundle, audience, time)
            .await
    }
}

impl JWTSVIDValidator {
    async fn validate_inner(
        &self,
        jwt_svid_compact: &str,
        trust_bundle: &TrustBundle,
        audience: &str,
        time: u64,
    ) -> Result<JWTSVID, Error> {
        let split = jwt_svid_compact.split('.').collect::<Vec<&str>>();

        if split.len() != 3 {
            return Err(Error::InvalidJoseEncoding(split.len()));
        }

        let data = format!("{}.{}", split[0], split[1]);

        let digest = sha::sha256(data.as_bytes());

        let jwtsvid_signature = split[2].to_string();

        let header_compact = base64::decode_config(split[0], base64::STANDARD_NO_PAD)
            .map_err(Error::InvalidBase64Encoding)?;
        let claim_compact = base64::decode_config(split[1], base64::STANDARD_NO_PAD)
            .map_err(Error::InvalidBase64Encoding)?;
        let signature_encrypted = base64::decode_config(split[2], base64::STANDARD_NO_PAD)
            .map_err(Error::InvalidBase64Encoding)?;

        let header_compact =
            std::str::from_utf8(&header_compact).map_err(Error::InvalidUTF8Encoding)?;
        let claim_compact =
            std::str::from_utf8(&claim_compact).map_err(Error::InvalidUTF8Encoding)?;

        let header: JWTHeader =
            serde_json::from_str(header_compact).map_err(Error::DeserializeJson)?;
        let claims: JWTClaims =
            serde_json::from_str(claim_compact).map_err(Error::DeserializeJson)?;

        if JWTType::JWT != header.jwt_type {
            return Err(Error::InvalidJWTType(header.jwt_type));
        }

        // Check token is not expired.
        if claims.expiry < time {
            return Err(Error::ExpiredToken {
                current: time,
                expiry: claims.expiry,
            });
        }

        let _: &String = claims
            .audience
            .iter()
            .find(|claims_audience| claims_audience == &audience)
            .ok_or_else(|| Error::InvalidAudience(audience.to_string()))?;

        let jwk = trust_bundle
            .jwt_key_set
            .keys
            .iter()
            .find(|jwk| jwk.kid == header.key_id)
            .ok_or_else(|| Error::PublicKeyNotInTrustBundle(header.key_id.clone()))?;

        match header.algorithm {
            KeyType::ES256 => {
                let ec_group = openssl::ec::EcGroup::from_curve_name(nid::Nid::X9_62_PRIME256V1)
                    .map_err(Error::ECGroupFromNID)?;

                let x = &base64::decode_config(jwk.x.clone(), base64::STANDARD_NO_PAD)
                    .map_err(Error::Base64DecodeCoordinates)?;
                let x = BigNum::from_slice(x).map_err(Error::BigNumberFromSlice)?;

                let y = &base64::decode_config(jwk.y.clone(), base64::STANDARD_NO_PAD)
                    .map_err(Error::Base64DecodeCoordinates)?;
                let y = BigNum::from_slice(y).map_err(Error::BigNumberFromSlice)?;

                let public_key =
                    openssl::ec::EcKey::from_public_key_affine_coordinates(&ec_group, &x, &y)
                        .map_err(Error::ECKeyFromPubKeyAffineCoordinates)?;

                let ecda_sign = openssl::ecdsa::EcdsaSig::from_der(&signature_encrypted)
                    .map_err(Error::CannotConvertSignatureToEcdsaSignature)?;

                ecda_sign
                    .verify(&digest, &public_key)
                    .map_err(Error::SignatureVerificationErrorEcdsa)?
                    .then(|| JWTSVID {
                        header,
                        claims,
                        signature: jwtsvid_signature,
                    })
                    .ok_or(Error::InvalidSignature)
            }
            _ => Err(Error::InvalidAlgorithm(header.algorithm)),
        }
    }
}

#[cfg(test)]
mod tests {
    use catalog::inmemory;
    use core_objects::CONFIG_DEFAULT_PATH;
    use key_manager::KeyManager;
    use key_store::disk;
    use matches::assert_matches;
    use server_config::{Config, KeyStoreConfig, KeyStoreConfigDisk};
    use std::sync::Arc;
    use svid_factory::{JWTSVIDParams, SVIDFactory};
    use tempdir::TempDir;
    use trust_bundle_builder::TrustBundleBuilder;

    use super::*;

    async fn init() -> (
        JWTSVIDValidator,
        SVIDFactory,
        TrustBundle,
        Config,
        Arc<KeyManager>,
    ) {
        let mut config = Config::load_config(CONFIG_DEFAULT_PATH).unwrap();
        let dir = TempDir::new("test").unwrap();
        let key_base_path = dir.into_path().to_str().unwrap().to_string();
        let key_plugin = KeyStoreConfigDisk {
            key_base_path: key_base_path.clone(),
        };

        // Change key disk plugin path to write in tempdir
        config.key_store = KeyStoreConfig::Disk(key_plugin.clone());
        // Force ttl to 10
        config.jwt.key_ttl = 10;

        let catalog = Arc::new(inmemory::Catalog::new());
        let key_store = Arc::new(disk::KeyStore::new(&key_plugin));

        let key_manager = Arc::new(
            KeyManager::new(&config, catalog.clone(), key_store.clone(), 0)
                .await
                .unwrap(),
        );
        let svid_factory = SVIDFactory::new(key_manager.clone(), &config);

        let svid_validator = JWTSVIDValidator::default();

        let trust_bundle = TrustBundleBuilder::new(&config, catalog)
            .build_trust_bundle(true, true)
            .await
            .unwrap();

        (
            svid_validator,
            svid_factory,
            trust_bundle,
            config,
            key_manager,
        )
    }

    #[tokio::test]
    async fn validate_happy_path() {
        let (svid_validator, svid_factory, trust_bundle, _config, _key_manager) = init().await;

        let jwt_svid_params = JWTSVIDParams {
            spiffe_id_path: "path".to_string(),
            audiences: vec!["myaudience".to_string()],
            other_identities: Vec::new(),
        };

        let jwt_svid = svid_factory.create_jwt_svid(jwt_svid_params).await.unwrap();

        svid_validator
            .validate_inner(&jwt_svid.token, &trust_bundle, "myaudience", 0)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn validate_invalid_signature() {
        let (svid_validator, svid_factory, trust_bundle, _config, _key_manager) = init().await;

        let jwt_svid_params = JWTSVIDParams {
            spiffe_id_path: "path".to_string(),
            audiences: vec!["myaudience".to_string()],
            other_identities: Vec::new(),
        };

        // Get token from a valid jwt
        let jwt_svid = svid_factory
            .create_jwt_svid(jwt_svid_params.clone())
            .await
            .unwrap();
        let token = jwt_svid.token.split('.').collect::<Vec<&str>>()[2];

        //Create a token and apply signature of previous jwt
        let jwt_svid_params = JWTSVIDParams {
            spiffe_id_path: "hack".to_string(),
            audiences: vec!["myaudience".to_string()],
            other_identities: Vec::new(),
        };

        let jwt_svid = svid_factory.create_jwt_svid(jwt_svid_params).await.unwrap();
        let jwt_svid = jwt_svid.token.split('.').collect::<Vec<&str>>();

        let jwt_svid = format!("{}.{}.{}", jwt_svid[0], jwt_svid[1], token);
        // Try to valida the signature taken from a valid token and applied to a new token with "hack" as destination.
        let error = svid_validator
            .validate_inner(&jwt_svid, &trust_bundle, "myaudience", 0)
            .await
            .unwrap_err();

        assert_matches!(error, Error::InvalidSignature);
    }

    #[tokio::test]
    async fn validate_invalid_token() {
        let (svid_validator, _svid_factory, trust_bundle, config, _key_manager) = init().await;
        let audience_spiffe_id = format!("{}/{}", &config.trust_domain, "myaudience");

        let error = svid_validator
            .validate_inner("dummy", &trust_bundle, &audience_spiffe_id.to_string(), 0)
            .await
            .unwrap_err();

        assert_matches!(error, Error::InvalidJoseEncoding(_));

        let error = svid_validator
            .validate_inner(
                "header.claim.token",
                &trust_bundle,
                &audience_spiffe_id.to_string(),
                0,
            )
            .await
            .unwrap_err();
        assert_matches!(error, Error::InvalidBase64Encoding(_));

        let header = base64::encode("header");
        let claim = base64::encode("claim");
        let token = base64::encode("dummy");
        let token = format!("{}.{}.{}", header, claim, token);
        let error = svid_validator
            .validate_inner(&token, &trust_bundle, &audience_spiffe_id.to_string(), 0)
            .await
            .unwrap_err();
        assert_matches!(error, Error::DeserializeJson(_));
    }

    #[tokio::test]
    async fn validate_expired() {
        let (svid_validator, svid_factory, trust_bundle, _config, _key_manager) = init().await;

        let jwt_svid_params = JWTSVIDParams {
            spiffe_id_path: "path".to_string(),
            audiences: vec!["myaudience".to_string()],
            other_identities: Vec::new(),
        };

        let jwt_svid = svid_factory.create_jwt_svid(jwt_svid_params).await.unwrap();

        let error = svid_validator
            .validate_inner(&jwt_svid.token, &trust_bundle, "myaudience", 12)
            .await
            .unwrap_err();
        assert_matches!(
            error,
            Error::ExpiredToken {
                expiry: _,
                current: _
            }
        );
    }

    #[tokio::test]
    async fn validate_jwt_invalid_audience() {
        let (svid_validator, svid_factory, trust_bundle, _config, _key_manager) = init().await;

        let jwt_svid_params = JWTSVIDParams {
            spiffe_id_path: "path".to_string(),
            audiences: vec!["myaudience".to_string()],
            other_identities: Vec::new(),
        };

        let jwt_svid = svid_factory.create_jwt_svid(jwt_svid_params).await.unwrap();

        let error = svid_validator
            .validate_inner(&jwt_svid.token, &trust_bundle, "wrongaudience", 0)
            .await
            .unwrap_err();
        assert_matches!(error, Error::InvalidAudience(_));
    }

    #[tokio::test]
    async fn validate_jwt_invalid_algorithm() {
        let (svid_validator, _svid_factory, trust_bundle, config, key_manager) = init().await;
        let slots = &*key_manager.slots.read().await;
        let jwt_key = &slots.current_jwt_key;

        let spiffe_id = format!("{}/{}", config.trust_domain, "path");
        let audience_spiffe_id = format!("{}/{}", config.trust_domain, "myaudience");

        let header = JWTHeader {
            algorithm: KeyType::PS512, //unimplemented algorithm
            key_id: jwt_key.id.clone(),
            jwt_type: JWTType::JWT,
        };

        let token = get_token(&header, spiffe_id.clone(), audience_spiffe_id.clone());

        let error = svid_validator
            .validate_inner(&token, &trust_bundle, &audience_spiffe_id, 0)
            .await
            .unwrap_err();
        assert_matches!(error, Error::InvalidAlgorithm(_));
    }

    #[tokio::test]
    async fn validate_jwt_invalid_kid() {
        let (svid_validator, _svid_factory, trust_bundle, config, key_manager) = init().await;
        let slots = &*key_manager.slots.read().await;
        let _jwt_key = &slots.current_jwt_key;

        let spiffe_id = format!("{}/{}", config.trust_domain, "path");
        let audience_spiffe_id = format!("{}/{}", config.trust_domain, "myaudience");

        let header = JWTHeader {
            algorithm: key_manager.jwt_key_type,
            key_id: "dummy".to_string(), //random kid
            jwt_type: JWTType::JWT,
        };

        let token = get_token(&header, spiffe_id.clone(), audience_spiffe_id.clone());

        let error = svid_validator
            .validate_inner(&token, &trust_bundle, &audience_spiffe_id, 0)
            .await
            .unwrap_err();
        assert_matches!(error, Error::PublicKeyNotInTrustBundle(_));
    }

    #[tokio::test]
    async fn validate_jwt_invalid_jwt_type() {
        let (svid_validator, _svid_factory, trust_bundle, config, key_manager) = init().await;
        let slots = &*key_manager.slots.read().await;
        let jwt_key = &slots.current_jwt_key;

        let spiffe_id = format!("{}/{}", config.trust_domain, "path");
        let audience_spiffe_id = format!("{}/{}", config.trust_domain, "myaudience");

        let header = JWTHeader {
            algorithm: key_manager.jwt_key_type,
            key_id: jwt_key.id.clone(),
            jwt_type: JWTType::JOSE,
        };

        let token = get_token(&header, spiffe_id.clone(), audience_spiffe_id.clone());

        let error = svid_validator
            .validate_inner(&token, &trust_bundle, &audience_spiffe_id, 0)
            .await
            .unwrap_err();
        assert_matches!(error, Error::InvalidJWTType(_));
    }

    fn get_token(header: &JWTHeader, spiffe_id: String, audience_spiffe_id: String) -> String {
        let claims = JWTClaims {
            subject: spiffe_id,
            audience: vec![audience_spiffe_id],
            expiry: 10,
            issued_at: 0,
            other_identities: Vec::new(),
        };

        let header_compact = serde_json::to_string(header).unwrap();
        let header_compact =
            base64::encode_config(header_compact.as_bytes(), base64::STANDARD_NO_PAD);

        let claims_compact = serde_json::to_string(&claims).unwrap();
        let claims_compact =
            base64::encode_config(claims_compact.as_bytes(), base64::STANDARD_NO_PAD);

        let dummy_signature =
            base64::encode_config("dummysignature".as_bytes(), base64::STANDARD_NO_PAD);

        format!("{}.{}.{}", header_compact, claims_compact, dummy_signature)
    }
}
