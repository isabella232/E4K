// Copyright (c) Microsoft. All rights reserved.

pub mod error;

use crate::Client as ClientTrait;

use agent_config::ServerConfig;
use error::Error;
use http_common::{Connector, ErrorBody, HttpRequest};
use server_agent_api::{create_workload_jwts, get_trust_bundle, ApiVersion};
use url::Url;

pub struct Client {
    connector: http_common::Connector,
    address_url: Url,
}

#[must_use]
pub fn create_workload_jwts_uri() -> String {
    format!("workload-jwts?api-version={}", ApiVersion::V2022_06_01)
}

#[must_use]
pub fn get_trust_bundle_uri() -> String {
    format!("trust-bundle?api-version={}", ApiVersion::V2022_06_01)
}

impl Client {
    pub fn new(server_config: &ServerConfig) -> Result<Self, Error> {
        let address_url = url::Url::parse(&format!(
            "http://{}:{}",
            server_config.address, server_config.port
        ))
        .map_err(Error::InvalidAddress)?;

        let connector = Connector::new(&address_url).map_err(Error::from)?;

        Ok(Self {
            connector,
            address_url,
        })
    }
}

#[async_trait::async_trait]
impl ClientTrait for Client {
    async fn create_workload_jwts(
        &self,
        request: create_workload_jwts::Request,
    ) -> Result<create_workload_jwts::Response, Box<dyn std::error::Error + Send>> {
        let address_url = format!("{}{}", self.address_url, &create_workload_jwts_uri(),);
        let request = HttpRequest::post(self.connector.clone(), &address_url, Some(request));

        let response = request
            .json_response()
            .await
            .map_err(|err| Box::new(Error::CreateWorkloadJWTs(err)) as _)?;

        response
            .parse::<create_workload_jwts::Response, ErrorBody<'_>>(&[hyper::StatusCode::CREATED])
            .map_err(|err| Box::new(Error::DeserializingCreateWorkloadJWTsResponse(err)) as _)
    }

    async fn get_trust_bundle(
        &self,
        params: get_trust_bundle::Params,
    ) -> Result<get_trust_bundle::Response, Box<dyn std::error::Error + Send>> {
        let address_url = format!(
            "{}{}&jwt_keys={}&x509_cas={}",
            self.address_url,
            &get_trust_bundle_uri(),
            params.jwt_keys,
            params.x509_cas,
        );
        let request: HttpRequest<(), _> = HttpRequest::get(self.connector.clone(), &address_url);

        let response = request
            .json_response()
            .await
            .map_err(|err| Box::new(Error::GetTrustBundle(err)) as _)?;

        response
            .parse::<get_trust_bundle::Response, ErrorBody<'_>>(&[hyper::StatusCode::CREATED])
            .map_err(|err| Box::new(Error::DeserializingGetTrustBundleResponse(err)) as _)
    }
}
