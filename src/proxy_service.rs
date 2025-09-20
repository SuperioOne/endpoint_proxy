use crate::route_config::{HttpMethod, RouteConfig};
use actix_web::{
  HttpRequest, HttpResponse,
  body::BoxBody,
  cookie::Cookie,
  dev::{Payload, Service, ServiceFactory, ServiceRequest, ServiceResponse, always_ready},
  error::PayloadError,
  web::Query,
};
use futures_util::{Stream, StreamExt};
use reqwest::Client;
use std::{collections::HashMap, convert::Infallible, pin::Pin, str::FromStr, sync::Arc};
use tracing::{debug, error, warn};

pub struct ProxyRouteServiceFactory {
  config: Arc<RouteConfig>,
  http_client: Client,
}

impl ServiceFactory<ServiceRequest> for ProxyRouteServiceFactory {
  type Response = ServiceResponse;
  type Error = Infallible;
  type Config = ();
  type Service = ProxyRouteService;
  type InitError = ();
  type Future = core::future::Ready<Result<Self::Service, Self::InitError>>;

  fn new_service(&self, _: Self::Config) -> Self::Future {
    let service = ProxyRouteService {
      config: self.config.clone(),
      http_client: self.http_client.clone(),
    };

    core::future::ready(Ok(service))
  }
}

impl ProxyRouteServiceFactory {
  #[inline]
  pub fn new(http_client: Client, proxy_config: Arc<RouteConfig>) -> Self {
    Self {
      config: proxy_config,
      http_client,
    }
  }
}

pub struct ProxyRouteService {
  config: Arc<RouteConfig>,
  http_client: reqwest::Client,
}

enum RemoteRequestError {
  PayloadError(PayloadError),
  ReqwestError(reqwest::Error),
}

impl Service<ServiceRequest> for ProxyRouteService {
  type Response = ServiceResponse;
  type Error = Infallible;
  type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + 'static>>;

  always_ready!();

  fn call(&self, req: ServiceRequest) -> Self::Future {
    let http_client = self.http_client.clone();
    let route_config = self.config.clone();

    Box::pin(ProxyRouteService::exec(req, http_client, route_config))
  }
}

impl ProxyRouteService {
  async fn exec(
    service_request: ServiceRequest,
    http_client: Client,
    route_config: Arc<RouteConfig>,
  ) -> Result<ServiceResponse, Infallible> {
    let (http_request, payload) = service_request.into_parts();

    let http_response =
      match Self::send_remote(http_client, &http_request, payload, route_config).await {
        Ok(success_response) => {
          debug!(message = "remote response", response = ?&success_response);
          success_response
        }
        Err(err) => {
          error!("Proxy request failed {}", err);
          HttpResponse::InternalServerError().finish()
        }
      };

    Ok(ServiceResponse::new(http_request, http_response))
  }

  async fn send_remote(
    http_client: Client,
    source_request: &HttpRequest,
    mut payload: Payload,
    route_config: Arc<RouteConfig>,
  ) -> Result<HttpResponse, RemoteRequestError> {
    let url: &str = &route_config.url;

    let mut remote_request = match route_config.method {
      Some(HttpMethod::Delete) => http_client.delete(url),
      Some(HttpMethod::Head) => http_client.head(url),
      Some(HttpMethod::Patch) => http_client.patch(url),
      Some(HttpMethod::Post) => http_client.post(url),
      Some(HttpMethod::Put) => http_client.put(url),
      _ => http_client.get(url),
    };

    let mut query_map =
      match Query::<HashMap<String, String>>::from_query(source_request.query_string()) {
        Ok(query_params) => query_params.0,
        Err(err) => {
          error!("Unable to parse query parameters {}", err);
          HashMap::new()
        }
      };

    if let Some(query_params) = route_config.query.as_ref() {
      for pair in query_params.iter() {
        if !query_map.contains_key(pair.name.as_ref()) {
          query_map.insert(pair.name.to_string(), pair.value.to_string());
        }
      }
    }

    if !query_map.is_empty() {
      remote_request = remote_request.query(&query_map);
    }

    if let Some(headers) = route_config.headers.as_deref() {
      let mut header_map = reqwest::header::HeaderMap::new();

      for header in headers.iter() {
        match (
          reqwest::header::HeaderName::from_str(&header.name),
          reqwest::header::HeaderValue::from_str(&header.value),
        ) {
          (Ok(name), Ok(value)) => _ = header_map.insert(name, value),
          (Err(err), _) => {
            error!(message = "ignoring invalid header name", reason= %err, source = source_request.path())
          }
          (_, Err(err)) => {
            error!(message = "ignoring invalid header value", reason= %err, source = source_request.path())
          }
        }
      }

      remote_request = remote_request.headers(header_map);
    }

    let (size, _) = payload.size_hint();
    let mut body_buffer: Vec<u8> = Vec::with_capacity(size);

    while let Some(chunk) = payload.next().await {
      body_buffer.extend_from_slice(chunk?.as_ref());
    }

    remote_request = if body_buffer.is_empty() {
      if let Some(default_body) = route_config.default_body.as_ref() {
        remote_request.body(default_body.as_bytes().to_owned())
      } else {
        remote_request
      }
    } else {
      remote_request.body(body_buffer)
    };

    let remote_response = remote_request.send().await?;
    Self::into_actix_response(remote_response).await
  }

  async fn into_actix_response(
    response: reqwest::Response,
  ) -> Result<HttpResponse, RemoteRequestError> {
    let status = actix_web::http::StatusCode::from_u16(response.status().as_u16())
      .expect("Unreachable error, mapping HTTP status codes must be fail safe");

    let mut http_response = HttpResponse::new(status);
    let headers = http_response.headers_mut();

    for (name, value) in response.headers() {
      match (
        actix_web::http::header::HeaderName::from_str(name.as_str()),
        actix_web::http::header::HeaderValue::from_bytes(value.as_bytes()),
      ) {
        (Ok(name), Ok(value)) => _ = headers.insert(name, value),
        (Err(err), _) => {
          warn!(message = "ignoring invalid header name", reason = %err)
        }
        (_, Err(err)) => {
          warn!(message = "ignoring invalid header value", reason = %err)
        }
      }
    }

    for cookie in response.cookies() {
      _ = http_response
        .add_cookie(&Cookie::new(cookie.name(), cookie.value()))
        .inspect_err(|err| {
          warn!(message = "Unable to set cookie", reason = %err);
        });
    }

    let body = response.bytes().await?;

    Ok(http_response.set_body(BoxBody::new(body)))
  }
}

impl From<PayloadError> for RemoteRequestError {
  #[inline]
  fn from(value: PayloadError) -> Self {
    Self::PayloadError(value)
  }
}

impl From<reqwest::Error> for RemoteRequestError {
  #[inline]
  fn from(value: reqwest::Error) -> Self {
    Self::ReqwestError(value)
  }
}

impl core::fmt::Display for RemoteRequestError {
  #[inline]
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    match self {
      RemoteRequestError::PayloadError(err) => err.fmt(f),
      RemoteRequestError::ReqwestError(err) => err.fmt(f),
    }
  }
}
