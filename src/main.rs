mod proxy_service;
mod route_config;

use crate::{
  proxy_service::ProxyRouteServiceFactory,
  route_config::{EndpointConfigFile, HttpMethod, RouteConfig},
};
use actix_cors::Cors;
use actix_web::{App, HttpServer, web};
use clap::Parser;
use reqwest::redirect::Policy;
use std::{fs, io::Result, num::NonZeroUsize, sync::Arc};
use tracing::{Level, debug, info};

#[derive(Parser, Debug)]
#[command(rename_all = "kebab-case")]
struct Config {
  #[arg(long, default_value_t = false)]
  enable_cookies: bool,
  #[arg(long, default_value_t = 8080)]
  port: u16,
  #[arg(long, default_value_t = std::thread::available_parallelism().map_or(2, NonZeroUsize::get))]
  worker_count: usize,
  #[arg(long, default_value_t = String::from("0.0.0.0"))]
  bind: String,
  #[arg(long, default_value_t = String::from("config.yaml"))]
  config_file: String,
  #[arg(long)]
  proxy_url: Option<String>,
  #[arg(long)]
  proxy_auth_user: Option<String>,
  #[arg(long)]
  proxy_auth_pass: Option<String>,
  #[arg(long, default_value_t = Level::INFO)]
  log_level: Level,
}

#[actix_web::main]
async fn main() -> Result<()> {
  let app_config = Config::parse();

  tracing_subscriber::fmt()
    .with_max_level(app_config.log_level)
    .init();

  debug!(message = "application config", config = ?app_config );

  let http_client = {
    let mut client_builder = reqwest::ClientBuilder::new();

    if let Some(proxy_url) = &app_config.proxy_url {
      let mut proxy = reqwest::Proxy::all(proxy_url).unwrap();

      if let (Some(user_name), Some(password)) =
        (&app_config.proxy_auth_user, &app_config.proxy_auth_pass)
      {
        proxy = proxy.basic_auth(user_name, password);
      }

      client_builder = client_builder.proxy(proxy);
    }

    if app_config.enable_cookies {
      client_builder = client_builder.cookie_store(true);
    }

    client_builder
      .redirect(Policy::limited(5))
      .build()
      .map_err(|err| std::io::Error::other(err.to_string()))?
  };

  let config_fd = fs::File::open(app_config.config_file)?;
  let route_configs: Vec<Arc<RouteConfig>> = EndpointConfigFile::load_from_file(&config_fd)?
    .proxy_urls
    .into_iter()
    .map(|e| Arc::from(e))
    .collect();

  HttpServer::new(move || {
    let cors = Cors::default()
      .allow_any_origin()
      .allow_any_header()
      .allow_any_method();

    let mut app = App::new().wrap(cors);

    for config in route_configs.iter() {
      let base_route = match config.method {
        Some(HttpMethod::Post) => web::post(),
        Some(HttpMethod::Put) => web::put(),
        Some(HttpMethod::Delete) => web::delete(),
        Some(HttpMethod::Head) => web::head(),
        Some(HttpMethod::Patch) => web::patch(),
        _ => web::get(),
      };

      let route_service = ProxyRouteServiceFactory::new(http_client.clone(), config.clone());

      app = app.route(&config.path, base_route.service(route_service));
      info!(message = "New route service", path = &config.path);
    }

    app
  })
  .workers(app_config.worker_count)
  .bind((app_config.bind, app_config.port))?
  .run()
  .await
}
