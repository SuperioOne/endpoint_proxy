use std::collections::HashMap;
use std::convert::Infallible;
use std::ops::Deref;
use std::sync::Arc;
use actix_web::{dev, HttpRequest, HttpResponse, ResponseError};
use actix_web::body::BoxBody;
use actix_web::cookie::Cookie;
use actix_web::dev::{Payload, Service, ServiceRequest, ServiceResponse};
use actix_web::web::Query;
use futures_core::future::{LocalBoxFuture};
use futures_core::Stream;
use futures_util::StreamExt;
use log::{debug, error, warn};
use reqwest::header::{HeaderMap};
use reqwest::{Client, RequestBuilder, Response};
use crate::proxy_service::proxy_config::ProxyConfig;
use crate::route_config::{HttpMethod};

pub struct ProxyRouteService {
  pub(super) config: Arc<ProxyConfig>,
  pub(super) http_client: Client,
}

impl Service<ServiceRequest> for ProxyRouteService {
  type Response = ServiceResponse;
  type Error = Infallible;
  type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

  dev::always_ready!();

  fn call(&self, req: ServiceRequest) -> Self::Future {
    let (http_request, payload) = req.into_parts();
    let proxy_request = self.init_request(&http_request);
    let default_body = self.config.default_body.clone();

    Box::pin(ProxyRouteService::exec(proxy_request, http_request, payload, default_body))
  }
}

impl ProxyRouteService {
  async fn exec(builder: RequestBuilder, http: HttpRequest, mut payload: Payload, default_body: Arc<[u8]>) -> Result<ServiceResponse, Infallible> {
    let proxy_response = {
      let (size, _) = payload.size_hint();
      let mut body_buffer: Vec<u8> = Vec::with_capacity(size);

      while let Some(chunk) = payload.next().await {
        match chunk {
          Ok(bytes) => {
            body_buffer.append(&mut bytes.to_vec());
          }
          Err(err) => {
            let error_response = err.error_response();
            return Ok(ServiceResponse::new(http, error_response));
          }
        }
      }

      if body_buffer.is_empty() {
        body_buffer = Vec::from(default_body.deref());
      }

      builder.body(body_buffer).send().await
    };

    debug!("Proxy response {:?}", &proxy_response);

    match proxy_response {
      Ok(data) => {
        let response = ProxyRouteService::map_response_head(&data);

        match data.bytes().await {
          Ok(bytes) => {
            Ok(ServiceResponse::new(http, response.set_body(BoxBody::new(bytes))))
          }
          Err(err) => {
            error!("Reading proxy body failed {}", err);
            let response = HttpResponse::InternalServerError().body("");
            Ok(ServiceResponse::new(http, response))
          }
        }
      }
      Err(err) => {
        error!("Proxy request failed {}", err);
        let response = HttpResponse::InternalServerError().body("");
        Ok(ServiceResponse::new(http, response))
      }
    }
  }

  fn init_request(&self, source_request: &HttpRequest) -> RequestBuilder {
    let mut builder = match self.config.method {
      HttpMethod::Get => self.http_client.get(self.config.url.as_ref()),
      HttpMethod::Post => self.http_client.post(self.config.url.as_ref()),
      HttpMethod::Put => self.http_client.put(self.config.url.as_ref()),
      HttpMethod::Delete => self.http_client.delete(self.config.url.as_ref()),
      HttpMethod::Head => self.http_client.head(self.config.url.as_ref()),
      HttpMethod::Patch => self.http_client.patch(self.config.url.as_ref()),
    };

    let mut query_map = match Query::<HashMap<String, String>>::from_query(source_request.query_string()) {
      Ok(query_params) => {
        query_params.0
      }
      Err(err) => {
        error!("Unable to parse query parameters {}", err);
        HashMap::new()
      }
    };

    if let Some(query_params) = self.config.query_params.as_deref() {
      for (name, value) in query_params {
        if !query_map.contains_key(name.as_ref()) {
          query_map.insert(name.to_string(), value.to_string());
        }
      }
    }

    if !query_map.is_empty() {
      builder = builder.query(&query_map);
    }

    if let Some(headers) = self.config.headers.as_deref() {
      let mut header_map = HeaderMap::new();

      for header in headers.iter() {
        header_map.insert(&header.name, header.value.clone());
      }

      builder = builder.headers(header_map);
    }

    builder
  }

  fn map_response_head(response: &Response) -> HttpResponse {
    let mut http_response = HttpResponse::new(response.status());
    let headers = http_response.headers_mut();

    for (name, value) in response.headers() {
      headers.insert(name.clone(), value.clone());
    }

    for cookie in response.cookies() {
      if let Err(cookie_error) = http_response.add_cookie(&Cookie::new(cookie.name(), cookie.value())) {
        warn!("Unable to set cookie for {}", cookie_error)
      }
    }

    http_response
  }
}