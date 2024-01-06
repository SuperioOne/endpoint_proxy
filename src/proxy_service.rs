use std::sync::Arc;
use reqwest::header::{HeaderName, HeaderValue};
use crate::proxy_service::proxy_config::{Header, ProxyConfig};
use crate::route_config::{NameValuePair, RouteConfig};

pub mod proxy_route_service;
pub mod proxy_factory;
pub mod proxy_config;

impl Into<ProxyConfig> for RouteConfig {
  fn into(self) -> ProxyConfig {
    let query_params = extract_query_params(&self);
    let default_body = extract_body(&self);
    let headers = extract_headers(&self);

    ProxyConfig {
      url: Box::from(self.url.as_str()),
      method: self.target_method.unwrap_or(self.method.unwrap_or_default()),
      query_params,
      default_body,
      headers,
    }
  }
}

#[inline]
fn extract_body(rule: &RouteConfig) -> Arc<[u8]> {
  let default_body = match &rule.default_body {
    Some(content) => {
      Arc::from(content.as_bytes())
    }
    None => {
      let empty: &[u8] = &[];
      Arc::from(empty)
    }
  };
  default_body
}

#[inline]
fn extract_query_params(rule: &RouteConfig) -> Option<Box<[(Box<str>, Box<str>)]>> {
  let query_params = match &rule.query {
    Some(values) => {
      let vec: Vec<(Box<str>, Box<str>)> = values
        .iter()
        .map(|pair| pair.into())
        .collect();

      Some(Box::from(vec.as_slice()))
    }
    None => None
  };
  query_params
}

#[inline]
fn extract_headers(rule: &RouteConfig) -> Option<Box<[Header]>> {
  let headers = match &rule.headers {
    Some(values) => {
      let vec: Vec<Header> = values
        .iter()
        .flat_map(|pair| pair.try_into())
        .collect();

      Some(Box::from(vec.as_slice()))
    }
    None => None
  };
  headers
}

impl Into<(Box<str>, Box<str>)> for &NameValuePair {
  fn into(self) -> (Box<str>, Box<str>) {
    let val_ptr: Box<str> = Box::from(self.value.as_str());
    let name_ptr: Box<str> = Box::from(self.name.as_str());

    (name_ptr, val_ptr)
  }
}

pub enum InvalidHeaderError {
  InvalidHeaderValue(String),
  InvalidHeaderName(String),
}

impl TryInto<Header> for &NameValuePair {
  type Error = InvalidHeaderError;

  fn try_into(self) -> Result<Header, Self::Error> {
    let value = HeaderValue::try_from(&self.value)
      .map_err(|e| InvalidHeaderError::InvalidHeaderValue(e.to_string()))?;

    let name = HeaderName::try_from(&self.name)
      .map_err(|e| InvalidHeaderError::InvalidHeaderName(e.to_string()))?;

    Ok(Header {
      name,
      value,
    })
  }
}