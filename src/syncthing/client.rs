use reqwest::header;

use super::errors::SyncthingError;
#[derive(Debug, Clone)]
pub struct Client {
    pub address: String,
    pub reqwester: reqwest::Client,
}

impl Client {
    pub fn new(auth_key: &String, address: &String, port: &u16) -> Self {
        let headers = Self::auth_header(auth_key);

        let full_address = format!("{}:{}", address, port);

        println!("full syncthing address: {}", full_address);

        match reqwest::Client::builder()
            .default_headers(headers)
            .build() {
                Ok(c) => Client {
                    address: full_address,
                    reqwester: c,
                },
                Err(_) => {
                    panic!("error building reqwest client");
                }
            }
    }

    pub fn auth_header(validated_auth_key: &String) -> header::HeaderMap {
        let api_key_header_val = match header::HeaderValue::from_str(validated_auth_key) {
            Ok(v) => v,
            Err(_e) => {
                panic!("error parsing api key while initiating Client struct")
            }
        };
        let mut headers = header::HeaderMap::new();
        headers.insert("X-API-KEY", api_key_header_val);
        headers
    }

    pub async fn get_events_since(&self, last_seen: &Option<u16>) -> Result<reqwest::Response, SyncthingError> {
        let url = match last_seen {
            Some(num) => format!("{}/rest/events?since={}", self.address, num.to_string()),
            None => format!("{}/rest/events", self.address),
        };

        let resp = self.reqwester.get(url).send().await?;

        Ok(resp)
    }
}