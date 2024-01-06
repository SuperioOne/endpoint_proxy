use std::sync::Arc;
use reqwest::header::{HeaderName, HeaderValue};
use crate::route_config::{HttpMethod};

#[derive(Clone)]
pub struct Header {
  pub name: HeaderName,
  pub value: HeaderValue,
}

pub struct ProxyConfig {
  pub url: Box<str>,
  pub headers: Option<Box<[Header]>>,
  pub query_params: Option<Box<[(Box<str>, Box<str>)]>>,
  pub method: HttpMethod,
  pub default_body: Arc<[u8]>,
}