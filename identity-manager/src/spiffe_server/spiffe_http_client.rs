use std::path::Path;

use http_common::{ErrorBody, HttpRequest};
use server_admin_api::RegistrationEntry;

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
        let response: server_admin_api::list_registration_entries::Response =
            response.parse_expect_ok::<_, ErrorBody<'_>>()?;

        let server_admin_api::list_registration_entries::Response {
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
            let mut response: server_admin_api::list_registration_entries::Response =
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
            response.parse::<_, ErrorBody<'_>>(hyper::StatusCode::CREATED)?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tempfile::{tempdir, TempDir};
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn connect_to_socket() {
        let server = start_test_server().await;
        let client = SpiffeHttpClient::new(&server.socket).expect("Could not make Spiffe Client");
        client.get_identities().await.expect("Can get identities");
    }

    #[tokio::test]
    async fn basic_crud() {
        let server = start_test_server().await;
        let client = SpiffeHttpClient::new(&server.socket).expect("Could not make Spiffe Client");

        // ======= get identities ====================================================================
        let current_identites = client.get_identities().await.expect("Can get identities");
        assert_eq!(0, current_identites.len());

        // ======= create identities ================================================================
        let modules_to_create: Vec<String> = (0..10).map(|i| format!("Module {}", i)).collect();
        let identities_to_create = modules_to_create
            .iter()
            .map(|id| RegistrationEntry {
                id: id.to_owned(),
                iot_hub_id: None,
                spiffe_id: "spiffe_id".to_owned(),
                parent_id: None,
                selectors: Vec::new(),
                admin: false,
                ttl: 1028,
                expires_at: 1028,
                dns_names: Vec::new(),
                revision_number: 0,
                store_svid: false,
            })
            .collect();

        client
            .create_identities(identities_to_create)
            .await
            .expect("Can create identities");

        let current_identites = client.get_identities().await.expect("Can get identities");
        assert_eq!(10, current_identites.len());

        let mut current_ids: Vec<String> = current_identites.iter().map(|e| e.id.clone()).collect();
        current_ids.sort();
        assert_eq!(modules_to_create, current_ids);

        // ======= delete identities ======================================================================
        let identities_to_delete: Vec<String> = (0..5).map(|i| format!("Module {}", i)).collect();
        client
            .delete_identities(identities_to_delete)
            .await
            .expect("Can delete identities");

        let current_identites = client.get_identities().await.expect("Can get identities");
        assert_eq!(5, current_identites.len());

        let mut current_ids: Vec<String> = current_identites.iter().map(|e| e.id.clone()).collect();
        current_ids.sort();
        let expected_ids: Vec<String> = (5..10).map(|i| format!("Module {}", i)).collect();
        assert_eq!(expected_ids, current_ids);
    }

    #[tokio::test]
    async fn paginated_get() {
        let server = start_test_server().await;
        let client = SpiffeHttpClient::new(&server.socket).expect("Could not make Spiffe Client");

        let current_identites = client.get_identities().await.expect("Can get identities");
        assert_eq!(0, current_identites.len());

        // create lots of identities
        let modules_to_create: Vec<String> = (0..1000).map(|i| format!("Module {}", i)).collect();
        let identities_to_create = modules_to_create
            .iter()
            .map(|id| RegistrationEntry {
                id: id.to_owned(),
                iot_hub_id: None,
                spiffe_id: "spiffe_id".to_owned(),
                parent_id: None,
                selectors: Vec::new(),
                admin: false,
                ttl: 1028,
                expires_at: 1028,
                dns_names: Vec::new(),
                revision_number: 0,
                store_svid: false,
            })
            .collect();
        client
            .create_identities(identities_to_create)
            .await
            .expect("Can create identities");

        // make sure we get all 1000 back
        let current_identites = client.get_identities().await.expect("Can get identities");
        assert_eq!(1000, current_identites.len());

        let mut current_ids: Vec<String> = current_identites.iter().map(|e| e.id.clone()).collect();
        current_ids.sort();
        let mut expected_ids = modules_to_create;
        expected_ids.sort();
        assert_eq!(expected_ids, current_ids);
    }

    struct TestServer {
        _dir: TempDir,
        pub socket: String,
    }

    async fn start_test_server() -> TestServer {
        let tmp_dir = tempdir().unwrap();
        let socket = tmp_dir.path().join("api.sock");
        let socket_string: String = socket.as_os_str().to_string_lossy().to_string();

        let server_socket_string = socket_string.clone(); // Need to clone to pass to new thread
        tokio::spawn(async move {
            let config = server_config::Config {
                socket_path: server_socket_string,
            };
            let catalog = Arc::new(catalog::inmemory::InMemoryCatalog::new());

            admin_api::start_admin_api(&config, catalog).await.unwrap();
        });
        sleep(Duration::from_millis(10)).await;

        TestServer {
            _dir: tmp_dir,
            socket: socket_string,
        }
    }
}
