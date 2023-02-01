use reqwest::header;
#[derive(Debug, Clone)]
pub struct Client {
    pub auth_key: String, 
}

impl Client {
    pub fn new(&self) -> reqwest::Client {
        let headers = self.auth_header(&self.auth_key);

        match reqwest::Client::builder()
            .default_headers(headers)
            .build() {
                Ok(c) => c,
                Err(_) => {
                    panic!("error building reqwest client");
                }
            }
    }

    fn auth_header(&self, validated_auth_key: &String) -> header::HeaderMap {
        let api_key_header_val = match header::HeaderValue::from_str(&validated_auth_key.to_string()) {
            Ok(v) => v,
            Err(_e) => {
                panic!("error parsing api key while initiating Client struct")
            }
        };
        let mut headers = header::HeaderMap::new();
        headers.insert("X-API-KEY", api_key_header_val);
        headers
    }
}