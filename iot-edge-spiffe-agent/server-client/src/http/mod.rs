// Copyright (c) Microsoft. All rights reserved.

pub mod error;

use crate::Client as ClientTrait;

use agent_config::ServerConfig;
use core_objects::{JWTSVIDCompact, SPIFFEID};
use error::Error;
use http_common::{Connector, ErrorBody, HttpRequest};
use server_agent_api::{attest_agent, create_workload_jwt, ApiVersion};
use url::Url;

pub struct Client {
    connector: http_common::Connector,
    address_url: Url,
}

#[must_use]
pub fn attest_agent_uri() -> String {
    format!("attest-agent?api-version={}", ApiVersion::V2022_06_01)
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
    async fn create_workload_jwt(
        &self,
        _request: create_workload_jwt::Request,
    ) -> Result<create_workload_jwt::Response, Box<dyn std::error::Error + Send>> {
        //!! TODO Place holder
        Ok(create_workload_jwt::Response {
            jwt_svid: JWTSVIDCompact {
                token: "dummy".to_string(),
                spiffe_id: SPIFFEID {
                    trust_domain: "dummy".to_string(),
                    path: "dummy".to_string(),
                },
                expiry: 0,
                issued_at: 0,
            },
        })
    }

    async fn attest_agent(
        &self,
        auth: attest_agent::Auth,
    ) -> Result<attest_agent::Response, Box<dyn std::error::Error + Send>> {
        let address_url = format!(
            "{}{}&token={}",
            self.address_url,
            &attest_agent_uri(),
            auth.token
        );
        let request: HttpRequest<(), _> = HttpRequest::get(self.connector.clone(), &address_url);

        let response = request
            .json_response()
            .await
            .map_err(|err| Box::new(Error::AttestAgent(err)) as _)?;

        response
            .parse::<attest_agent::Response, ErrorBody<'_>>(&[hyper::StatusCode::CREATED])
            .map_err(|err| Box::new(Error::DeserializingAttestAgentResponse(err)) as _)
    }
}
