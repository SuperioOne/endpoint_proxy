use std::ops::Deref;
use std::str::FromStr;
use std::sync::Arc;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::{Error, Response};
use crate::http_client_provider;

pub struct ProxyItem {
    url: Arc<str>,
    headers: Option<Arc<[(HeaderName, HeaderValue)]>>,
    query_params: Option<Arc<[(String, String)]>>,
}

type KeyValue<'a> = (&'a str, &'a str);

impl ProxyItem {
    pub fn new(url: &str) -> ProxyItem {
        ProxyItem {
            url: Arc::from(url),
            headers: None,
            query_params: None,
        }
    }

    pub fn set_headers<'a, I>(mut self, headers: I) -> Self where I: IntoIterator<Item=KeyValue<'a>> {
        let buffer: Vec<(HeaderName, HeaderValue)> = headers.into_iter()
            .map(|(key, value)| {
                let header_info = (HeaderName::from_str(key), HeaderValue::from_str(value));
                match header_info {
                    (Ok(key), Ok(value)) => Some((key, value)),
                    _ => None
                }
            })
            .flatten()
            .collect();

        self.headers = Some(Arc::from(buffer));
        self
    }

    pub fn set_query<'a, I>(mut self, query: I) -> Self where I: IntoIterator<Item=KeyValue<'a>> {
        let buffer: Vec<(String, String)> = query.into_iter()
            .map(|(key, value)| {
                (String::from(key), String::from(value))
            })
            .collect();

        self.query_params = Some(Arc::from(buffer));
        self
    }

    pub async fn get(&self) -> std::result::Result<Response, Error> {
        let mut builder = http_client_provider::get_client()
            .get(self.url.deref());

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

        return builder.send().await;
    }
}