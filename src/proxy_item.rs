use std::ops::Deref;
use std::str::FromStr;
use std::sync::Arc;

use log::warn;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::{Error, Response};

use crate::{http_client_provider, HttpMethod};

pub struct ProxyItem {
    url: Arc<str>,
    headers: Option<Arc<[(HeaderName, HeaderValue)]>>,
    query_params: Option<Arc<[(String, String)]>>,
    method: HttpMethod,
    default_body: Option<Arc<[u8]>>,
}

type KeyValue<'a> = (&'a str, &'a str);

impl ProxyItem {
    pub fn new(url: &str) -> ProxyItem {
        ProxyItem {
            url: Arc::from(url),
            method: HttpMethod::Get,
            headers: None,
            query_params: None,
            default_body: None,
        }
    }

    pub fn set_default_body(mut self, bytes: &[u8]) -> Self {
        self.default_body = Some(Arc::from(bytes));
        self
    }

    pub fn set_headers<'a, I>(mut self, headers: I) -> Self
    where
        I: IntoIterator<Item = KeyValue<'a>>,
    {
        let buffer: Vec<(HeaderName, HeaderValue)> = headers
            .into_iter()
            .map(|(key, value)| {
                let header_info = (HeaderName::from_str(key), HeaderValue::from_str(value));
                match header_info {
                    (Ok(key), Ok(value)) => Some((key, value)),
                    (Err(error), _) => {
                        warn!(
                            "Ignoring invalid header name '{}'. Details: {}",
                            key,
                            error.to_string()
                        );
                        None
                    }
                    (_, Err(error)) => {
                        warn!(
                            "Ignoring invalid header value '{}' for the key '{}'. Details: {}",
                            value,
                            key,
                            error.to_string()
                        );
                        None
                    }
                }
            })
            .flatten()
            .collect();

        self.headers = Some(Arc::from(buffer));
        self
    }

    pub fn set_query<'a, I>(mut self, query: I) -> Self
    where
        I: IntoIterator<Item = KeyValue<'a>>,
    {
        let buffer: Vec<(String, String)> = query
            .into_iter()
            .map(|(key, value)| (String::from(key), String::from(value)))
            .collect();

        self.query_params = Some(Arc::from(buffer));
        self
    }

    pub fn set_method(mut self, method: HttpMethod) -> Self {
        self.method = method;
        self
    }

    pub async fn execute(&self, body: bytes::Bytes) -> Result<Response, Error> {
        let client = http_client_provider::get_client();
        let mut builder = match self.method {
            HttpMethod::Get => client.get(self.url.deref()),
            HttpMethod::Post => client.post(self.url.deref()),
            HttpMethod::Put => client.put(self.url.deref()),
            HttpMethod::Delete => client.delete(self.url.deref()),
            HttpMethod::Head => client.head(self.url.deref()),
            HttpMethod::Patch => client.patch(self.url.deref()),
        };

        if let Some(a) = &self.query_params {
            builder = builder.query(a.deref());
        }

        if let Some(a) = &self.headers {
            let mut header_map = HeaderMap::new();

            for (key, value) in a.iter() {
                header_map.append(key, value.clone());
            }

            builder = builder.headers(header_map);
        }
        return if body.len() > 0 {
            builder.body(reqwest::Body::from(body)).send().await
        } else if let Some(default_body) = &self.default_body {
            let body_clone = Vec::from(default_body.deref());
            builder.body(reqwest::Body::from(body_clone)).send().await
        } else {
            builder.send().await
        };
    }
}
