// Copyright (c) Microsoft. All rights reserved.

use core_objects::RegistrationEntry;
use http_common::{ErrorBody, HttpRequest};

pub use super::SpiffeConnector;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync + 'static>>;
const BASE_URL: &str = "https://spiffieserver.sock/entries?api-version=2022-06-01";

pub struct SpiffeHttpClient {
    connector: http_common::Connector,
}

impl SpiffeHttpClient {
    pub fn new(socket: &str) -> Result<Self> {
        let socket_url = url::Url::parse(&format!("unix://{}", socket))?;

        let connector = http_common::Connector::new(&socket_url)
            .map_err(|err| format!("Could not make connector: {:#?}", err))?;

        Ok(Self { connector })
    }
}

#[async_trait::async_trait]
impl SpiffeConnector for SpiffeHttpClient {
    async fn get_identities(&self) -> Result<Vec<RegistrationEntry>> {
        let uri = format!("{}&page_size=20", BASE_URL);
        let request: HttpRequest<(), _> = HttpRequest::get(self.connector.clone(), &uri);

        let response = request.json_response().await?;
        let response: server_admin_api::list_all::Response =
            response.parse_expect_ok::<_, ErrorBody<'_>>()?;

        let server_admin_api::list_all::Response {
            mut entries,
            mut next_page_token,
        } = response;

        while let Some(page_token) = &next_page_token {
            let page_token = percent_encoding::percent_encode(
                page_token.as_bytes(),
                http_common::PATH_SEGMENT_ENCODE_SET,
            );
            let uri = format!("{}&page_size=20&page_token={}", BASE_URL, page_token);
            let request: HttpRequest<(), _> = HttpRequest::get(self.connector.clone(), &uri);

            let response = request.json_response().await?;
            let mut response: server_admin_api::list_all::Response =
                response.parse_expect_ok::<_, ErrorBody<'_>>()?;

            entries.append(&mut response.entries);
            next_page_token = response.next_page_token;
        }

        Ok(entries)
    }

    async fn create_identities(&self, identities_to_create: Vec<RegistrationEntry>) -> Result<()> {
        let body = server_admin_api::update_registration_entries::Request {
            entries: identities_to_create,
        };

        let request = HttpRequest::post(self.connector.clone(), BASE_URL, Some(body));
        let response = request.json_response().await?;
        let _response: server_admin_api::update_registration_entries::Response =
            response.parse::<_, ErrorBody<'_>>(&[hyper::StatusCode::CREATED])?;

        // TODO: check response for all created? will server not send error response?

        Ok(())
    }

    async fn delete_identities(&self, identities_to_delete: Vec<String>) -> Result<()> {
        let body = server_admin_api::delete_registration_entries::Request {
            ids: identities_to_delete,
        };

        let request = HttpRequest::delete(self.connector.clone(), BASE_URL, Some(body));
        let response = request.json_response().await?;
        let _response: server_admin_api::update_registration_entries::Response =
            response.parse_expect_ok::<_, ErrorBody<'_>>()?;

        Ok(())
    }
}
