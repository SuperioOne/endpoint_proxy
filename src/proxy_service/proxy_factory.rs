use std::convert::Infallible;
use std::sync::Arc;
use actix_web::dev::{ServiceFactory, ServiceRequest, ServiceResponse};
use futures_core::future::LocalBoxFuture;
use reqwest::Client;
use crate::proxy_service::proxy_config::ProxyConfig;
use crate::proxy_service::proxy_route_service::ProxyRouteService;

pub struct ProxyRouteServiceFactory {
  pub config: Arc<ProxyConfig>,
  pub http_client: Client,
}

impl ServiceFactory<ServiceRequest> for ProxyRouteServiceFactory {
  type Response = ServiceResponse;
  type Error = Infallible;
  type Config = ();
  type Service = ProxyRouteService;
  type InitError = ();
  type Future = LocalBoxFuture<'static, Result<Self::Service, Self::InitError>>;

  fn new_service(&self, _: Self::Config) -> Self::Future {
    let service = ProxyRouteService {
      config: self.config.clone(),
      http_client: self.http_client.clone(),
    };

    Box::pin(async move { Ok(service) })
  }
}

impl ProxyRouteServiceFactory {
  pub fn create(http_client: Client, proxy_config: Arc<ProxyConfig>) -> Self {
    Self {
      config: proxy_config,
      http_client,
    }
  }
}
