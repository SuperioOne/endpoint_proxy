use crate::proxy_service::proxy_config::{Header, ProxyConfig};
use crate::route_config::{NameValuePair, RouteConfig};
use actix_web::http::header::InvalidHeaderName;
use reqwest::header::{HeaderName, HeaderValue, InvalidHeaderValue};
use std::sync::Arc;

pub mod proxy_config;
pub mod proxy_factory;
pub mod proxy_route_service;

pub enum InvalidHeaderError {
  InvalidHeaderValue(String),
  InvalidHeaderName(String),
}

impl From<RouteConfig> for ProxyConfig {
  fn from(value: RouteConfig) -> Self {
    let query_params = extract_query_params(value.query);
    let default_body = extract_body(value.default_body);
    let headers = extract_headers(value.headers);
    let method = value
      .target_method
      .unwrap_or(value.method.unwrap_or_default());

    ProxyConfig {
      url: value.url.into_boxed_str(),
      method,
      query_params,
      default_body,
      headers,
    }
  }
}

#[inline]
fn extract_body(body: Option<String>) -> Arc<[u8]> {
  let default_body = match body {
    Some(content) => Arc::from(content.as_bytes()),
    None => {
      let empty: &[u8] = &[];
      Arc::from(empty)
    }
  };
  default_body
}

#[inline]
fn extract_query_params(
  query_params: Option<Vec<NameValuePair>>,
) -> Option<Box<[(Box<str>, Box<str>)]>> {
  let query_params = match query_params {
    Some(values) => {
      let vec: Vec<(Box<str>, Box<str>)> = values.into_iter().map(|pair| pair.into()).collect();

      Some(Box::from(vec.as_slice()))
    }
    None => None,
  };
  query_params
}

#[inline]
fn extract_headers(headers: Option<Vec<NameValuePair>>) -> Option<Box<[Header]>> {
  let headers = match headers {
    Some(values) => {
      let vec: Vec<Header> = values
        .into_iter()
        .flat_map(|pair| pair.try_into())
        .collect();

      Some(Box::from(vec.as_slice()))
    }
    None => None,
  };
  headers
}

impl From<NameValuePair> for (Box<str>, Box<str>) {
  fn from(value: NameValuePair) -> Self {
    let name_ptr: Box<str> = value.name.into_boxed_str();
    let val_ptr: Box<str> = value.value.into_boxed_str();

    (name_ptr, val_ptr)
  }
}

impl TryFrom<NameValuePair> for Header {
  type Error = InvalidHeaderError;

  fn try_from(value: NameValuePair) -> Result<Self, Self::Error> {
    let header_value = HeaderValue::try_from(value.value)?;
    let header_name = HeaderName::try_from(value.name)?;

    Ok(Header {
      name: header_name,
      value: header_value,
    })
  }
}

impl From<InvalidHeaderValue> for InvalidHeaderError {
  fn from(value: InvalidHeaderValue) -> Self {
    InvalidHeaderError::InvalidHeaderValue(value.to_string())
  }
}

impl From<InvalidHeaderName> for InvalidHeaderError {
  fn from(value: InvalidHeaderName) -> Self {
    InvalidHeaderError::InvalidHeaderName(value.to_string())
  }
}
