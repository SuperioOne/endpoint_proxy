mod http_client_provider;
mod proxy_item;
mod std_logger;

use std::{env, fs};
use std::collections::HashMap;
use actix_web::{App, HttpRequest, HttpResponse, HttpServer, Responder, web};
use std::io::{ErrorKind, Result};
use std::str::FromStr;
use std::sync::Arc;
use log::{error, info, LevelFilter, warn};
use serde::{Deserialize, Serialize};
use crate::proxy_item::{ProxyItem};
use crate::std_logger::StdLogger;

static LOGGER: StdLogger = StdLogger;

struct Config {
    proxy_cookies: bool,
    port: u16,
    worker_count: usize,
    bind: String,
    config_file: String,
    proxy_url: Option<String>,
    proxy_auth_user: Option<String>,
    proxy_auth_pass: Option<String>,
    log_level: LevelFilter,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct ValuePair {
    name: String,
    value: String,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct ConfigProxyConfig {
    path: String,
    url: String,
    query: Option<Vec<ValuePair>>,
    headers: Option<Vec<ValuePair>>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct ConfigFile {
    proxy_urls: Vec<ConfigProxyConfig>,
}

struct AppState {
    endpoint_map: HashMap<Arc<str>, Arc<ProxyItem>>,
}

#[actix_web::main]
async fn main() -> Result<()> {
    let config: Config = read_env_vars();
    let _ = log::set_logger(&LOGGER)
        .map(|()| log::set_max_level(config.log_level));

    info!("Log level set to {}", &config.log_level);
    info!("Worker count set to {}", &config.worker_count);
    info!("Server bind address set to '{}'",  &config.bind);
    info!("Server port to '{}'", &config.port);
    info!("Route configuration file path set to '{}'", &config.config_file);

    let http_client_config = Some(http_client_provider::ClientConfig {
        http_proxy: config.proxy_url,
        pass: config.proxy_auth_pass,
        user: config.proxy_auth_user,
        enable_cookies: config.proxy_cookies,
    });

    http_client_provider::build(http_client_config)
        .map_err(|error| { std::io::Error::new(ErrorKind::Other, error) })?;

    let config_fd = fs::File::open(config.config_file)?;
    let path_configs: ConfigFile = serde_yaml::from_reader(config_fd)
        .map_err(|err| std::io::Error::new(ErrorKind::Other, err))?;

    let endpoint_store: HashMap<Arc<str>, Arc<ProxyItem>> = HashMap::from_config(path_configs);

    HttpServer::new(move || {
        let mut app = App::new();

        for (key, _) in endpoint_store.iter() {
            app = app.route(&key, web::get().to(handler));
        }

        app.app_data(web::Data::new(AppState {
            endpoint_map: endpoint_store.clone()
        }))
    })
        .workers(config.worker_count)
        .bind((config.bind, config.port))?
        .run()
        .await
}

async fn handler(req: HttpRequest, data: web::Data<AppState>) -> impl Responder {
    let key = req.path();
    if let Some(proxy_item) = data.endpoint_map.get(key)
    {
        let response = proxy_item.get().await;
        match response {
            Ok(success_response) => {
                let mut result = HttpResponse::Ok();

                for header in success_response.headers().iter() {
                    result.append_header(header);
                }

                let body_stream = success_response.bytes_stream();
                result.streaming(body_stream)
            }
            Err(err) => {
                error!("Proxy route failed for '{}'. {}",key, err.status().map_or("".into(), |e| e.to_string()));
                HttpResponse::BadRequest().finish()
            }
        }
    } else {
        warn!("Path '{}' is initialized but endpoint map is missing.", key);
        HttpResponse::NotFound().finish()
    }
}

trait EndpointStore {
    fn from_config(config: ConfigFile) -> Self;
}

impl EndpointStore for HashMap<Arc<str>, Arc<ProxyItem>> {
    fn from_config(config: ConfigFile) -> Self {
        let mut endpoint_store: HashMap<Arc<str>, Arc<ProxyItem>> = HashMap::new();
        for ConfigProxyConfig { query, url, path, headers } in config.proxy_urls.into_iter() {
            let mut proxy_item = ProxyItem::new(&url);

            if let Some(query_config) = query {
                let query_params = query_config
                    .iter()
                    .map(|e| (e.name.as_str(), e.value.as_str()));

                proxy_item = proxy_item.set_query(query_params);
            }

            if let Some(header_config) = headers {
                let header_params = header_config
                    .iter()
                    .map(|e| (e.name.as_str(), e.value.as_str()));

                proxy_item = proxy_item.set_headers(header_params);
            }

            let key: Arc<str> = Arc::from(path);
            let value: Arc<ProxyItem> = Arc::from(proxy_item);
            info!("New endpoint created at '{}'.", &key);
            endpoint_store.insert(key, value);
        }

        endpoint_store
    }
}

fn read_env_vars() -> Config
{
    #[cfg(debug_assertions)]
    const DEFAULT_LOG_LEVEL: LevelFilter = LevelFilter::Debug;
    #[cfg(not(debug_assertions))]
    const DEFAULT_LOG_LEVEL: LevelFilter = LevelFilter::Info;

    const DEFAULT_PORT: u16 = 8080;
    const DEFAULT_WORKER_COUNT: usize = 4;
    const DEFAULT_BIND: &str = "0.0.0.0";

    let log_level: LevelFilter = env::var("LOG_LEVEL").map_or(DEFAULT_LOG_LEVEL, |e| LevelFilter::from_str(&e).unwrap_or(DEFAULT_LOG_LEVEL));
    let bind: String = env::var("HTTP_BIND").map_or(DEFAULT_BIND.into(), |e| e);
    let port = env::var("HTTP_PORT").map_or(DEFAULT_PORT, |e| e.parse::<u16>().unwrap_or(DEFAULT_PORT));
    let proxy_url = env::var("HTTP_PROXY_URL").map_or(None, |e| Some(e));
    let proxy_auth_user = env::var("HTTP_PROXY_USER").map_or(None, |e| Some(e));
    let proxy_auth_pass = env::var("HTTP_PROXY_PASS").map_or(None, |e| Some(e));
    let proxy_cookies = env::var("HTTP_PROXY_COOKIES").map_or(false, |e| e.parse::<bool>().unwrap_or(false));
    let worker_count = env::var("HTTP_WORKER_COUNT").map_or(DEFAULT_WORKER_COUNT, |e| e.parse::<usize>().unwrap_or(DEFAULT_WORKER_COUNT));
    let config_file = env::var("ROUTE_CONF_LOCATION").map_or("route_config.yaml".into(), |e| e);

    Config {
        worker_count,
        proxy_cookies,
        proxy_url,
        proxy_auth_user,
        proxy_auth_pass,
        port,
        bind,
        config_file,
        log_level,
    }
}

