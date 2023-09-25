mod http_client_provider;
mod proxy_item;

use std::{env, fs};
use std::collections::HashMap;
use actix_web::{App, HttpRequest, HttpResponse, HttpServer, Responder, web};
use std::io::{ErrorKind, Result};
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use proxy_item::{ProxyItem};

struct Config {
    proxy_cookies: bool,
    port: u16,
    worker_count: usize,
    bind: String,
    config_file: String,
    proxy_url: Option<String>,
    proxy_auth_user: Option<String>,
    proxy_auth_pass: Option<String>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct ConfigQueryInfo {
    name: String,
    value: String,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct ConfigProxyConfig {
    path: String,
    url: String,
    query: Option<Vec<ConfigQueryInfo>>,
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

    let mut store: HashMap<Arc<str>, Arc<ProxyItem>> = HashMap::new();

    for ConfigProxyConfig { query, url, path } in path_configs.proxy_urls.into_iter() {
        let mut proxy_item = ProxyItem::new(&url);
        if let Some(query_config) = query {
            let query_params = query_config
                .iter()
                .map(|e| (e.name.as_str(), e.value.as_str()));

            proxy_item = proxy_item.set_query(query_params);
        }

        let key: Arc<str> = Arc::from(path);
        let value: Arc<ProxyItem> = Arc::from(proxy_item);
        store.insert(key, value);
    }

    HttpServer::new(move || {
        let mut app = App::new();

        for (key, _) in store.iter() {
            app = app.route(&key, web::get().to(handler));
        }

        app.app_data(web::Data::new(AppState {
            endpoint_map: store.clone()
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
                eprintln!("{:?}", err);
                HttpResponse::BadRequest().finish()
            }
        }
    } else {
        HttpResponse::NotFound().finish()
    }
}

fn read_env_vars() -> Config
{
    const DEFAULT_PORT: u16 = 8080;
    const DEFAULT_WORKER_COUNT: usize = 4;
    const DEFAULT_BIND: &str = "0.0.0.0";

    let bind: String = env::var("HTTP_BIND").map_or(DEFAULT_BIND.into(), |e| e);
    let port = env::var("HTTP_PORT").map_or(DEFAULT_PORT, |e| e.parse::<u16>().unwrap_or(DEFAULT_PORT));
    let proxy_url = env::var("HTTP_PROXY_URL").map_or(None, |e| Some(e));
    let proxy_auth_user = env::var("HTTP_PROXY_USER").map_or(None, |e| Some(e));
    let proxy_auth_pass = env::var("HTTP_PROXY_PASS").map_or(None, |e| Some(e));
    let proxy_cookies = env::var("HTTP_PROXY_COOKIES").map_or(false, |e| e.parse::<bool>().unwrap_or(false));
    let worker_count = env::var("HTTP_WORKER_COUNT").map_or(DEFAULT_WORKER_COUNT, |e| e.parse::<usize>().unwrap_or(DEFAULT_WORKER_COUNT));
    let config_file = env::var("ROUTE_CONF_LOCATION").map_or("config.yaml".into(), |e| e);

    Config {
        worker_count,
        proxy_cookies,
        proxy_url,
        proxy_auth_user,
        proxy_auth_pass,
        port,
        bind,
        config_file,
    }
}

