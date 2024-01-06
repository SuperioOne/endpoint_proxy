mod std_logger;
mod route_config;
mod proxy_service;
pub mod http_client;

use crate::std_logger::StdLogger;
use actix_cors::Cors;
use actix_web::{App, HttpServer, web};
use clap::Parser;
use log::{info, warn, LevelFilter};
use std::fmt::{Display, Formatter};
use std::fs;
use std::io::{ErrorKind, Result};
use std::num::NonZeroUsize;
use std::sync::Arc;
use crate::proxy_service::proxy_config::ProxyConfig;
use crate::proxy_service::proxy_factory::ProxyRouteServiceFactory;
use crate::route_config::{EndpointConfigFile, HttpMethod};

static LOGGER: StdLogger = StdLogger;

#[derive(Clone, Debug, Copy)]
struct LevelFilterArg(LevelFilter);

impl Default for LevelFilterArg {
  fn default() -> Self {
    LevelFilterArg(LevelFilter::Info)
  }
}

impl Display for LevelFilterArg {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.0)
  }
}

impl Into<LevelFilter> for LevelFilterArg {
  fn into(self) -> LevelFilter {
    self.0
  }
}

impl From<&str> for LevelFilterArg {
  fn from(value: &str) -> Self {
    match value.to_uppercase().as_str() {
      "INFO" => LevelFilterArg(LevelFilter::Info),
      "DEBUG" => LevelFilterArg(LevelFilter::Debug),
      "ERROR" => LevelFilterArg(LevelFilter::Error),
      "WARN" => LevelFilterArg(LevelFilter::Warn),
      "OFF" => LevelFilterArg(LevelFilter::Off),
      "TRACE" => LevelFilterArg(LevelFilter::Trace),
      value => {
        warn!("Unexpected log level '{}', falling back to INFO level.", value);
        LevelFilterArg(LevelFilter::Info)
      }
    }
  }
}

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
  #[arg(long, default_value_t = LevelFilterArg::default())]
  log_level: LevelFilterArg,
}

#[derive(Clone)]
struct ConfigItem {
  path: String,
  method: HttpMethod,
  proxy_config: Arc<ProxyConfig>,
}

#[actix_web::main]
async fn main() -> Result<()> {
  let config = Config::parse();
  let _ = log::set_logger(&LOGGER).map(|()| log::set_max_level(config.log_level.into()));

  info!("Log level set to {}", &config.log_level);
  info!("Worker count set to {}", &config.worker_count);
  info!("Server bind address set to '{}'", &config.bind);
  info!("Server port to '{}'", &config.port);
  info!("Cookie store is {}",
    if config.enable_cookies {"enabled"}
    else {"disabled"}
  );
  info!("Route configuration file path set to '{}'", &config.config_file);

  let http_client_config = http_client::HttpClientConfig {
    http_proxy: config.proxy_url,
    pass: config.proxy_auth_pass,
    user: config.proxy_auth_user,
    enable_cookies: config.enable_cookies,
  };

  let http_client = http_client_config
    .to_client()
    .map_err(|error| std::io::Error::new(ErrorKind::Other, error))?;

  let config_fd = fs::File::open(config.config_file)?;
  let config_file = EndpointConfigFile::load_from_file(&config_fd)?;
  let proxy_configs: Vec<ConfigItem> = config_file.proxy_urls
    .into_iter()
    .map(|e| {
      let path = e.path.clone();
      let method = e.method.unwrap_or_default();
      let proxy_config: ProxyConfig = e.into();

      ConfigItem {
        method,
        path,
        proxy_config: Arc::from(proxy_config),
      }
    })
    .collect();

  HttpServer::new(move || {
    // Clones configs for each worker
    let proxies = proxy_configs.clone();

    let cors = Cors::default()
      .allow_any_origin()
      .allow_any_header()
      .allow_any_method();

    let mut app = App::new().wrap(cors);

    for config in proxies.into_iter() {
      let base_route = match config.method {
        HttpMethod::Post => web::post(),
        HttpMethod::Put => web::put(),
        HttpMethod::Delete => web::delete(),
        HttpMethod::Head => web::head(),
        HttpMethod::Patch => web::patch(),
        _ => web::get()
      };

      let route_factory = ProxyRouteServiceFactory::create(http_client.clone(), config.proxy_config);
      app = app.route(&config.path, base_route.service(route_factory));

      info!("Route service set for {}", config.path);
    }

    app
  })
    .workers(config.worker_count)
    .bind((config.bind, config.port))?
    .run()
    .await
}
