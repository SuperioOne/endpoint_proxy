mod endpoint_map;
mod http_client_provider;
mod proxy_item;
mod std_logger;

use crate::proxy_item::ProxyItem;
use crate::std_logger::StdLogger;
use actix_cors::Cors;
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use clap::Parser;
use endpoint_map::*;
use log::{error, info, warn, LevelFilter};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::fs;
use std::io::{ErrorKind, Result};
use std::num::NonZeroUsize;
use std::ops::Deref;
use std::sync::Arc;

static LOGGER: StdLogger = StdLogger;

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Head,
    Patch,
}

impl TryFrom<&str> for HttpMethod {
    type Error = ();

    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "get" => Ok(HttpMethod::Get),
            "post" => Ok(HttpMethod::Post),
            "patch" => Ok(HttpMethod::Patch),
            "put" => Ok(HttpMethod::Put),
            "head" => Ok(HttpMethod::Head),
            "delete" => Ok(HttpMethod::Delete),
            _ => Err(()),
        }
    }
}

impl ToString for HttpMethod {
    fn to_string(&self) -> String {
        match self {
            HttpMethod::Get => String::from("get"),
            HttpMethod::Post => String::from("post"),
            HttpMethod::Put => String::from("put"),
            HttpMethod::Delete => String::from("delete"),
            HttpMethod::Head => String::from("head"),
            HttpMethod::Patch => String::from("patch"),
        }
    }
}

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
                warn!(
                    "Unexpected log level '{}', falling back to INFO level.",
                    value
                );
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

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct NameValuePair {
    name: String,
    value: String,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct ConfigProxyConfig {
    path: String,
    url: String,
    query: Option<Vec<NameValuePair>>,
    headers: Option<Vec<NameValuePair>>,
    method: Option<HttpMethod>,
    target_method: Option<HttpMethod>,
    default_body: Option<String>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct EndpointConfigFile {
    proxy_urls: Vec<ConfigProxyConfig>,
}

struct AppState {
    endpoint_map: HashMap<EndpointKey, Arc<ProxyItem>>,
}

#[actix_web::main]
async fn main() -> Result<()> {
    let config = Config::parse();

    let _ = log::set_logger(&LOGGER).map(|()| log::set_max_level(config.log_level.into()));

    info!("Log level set to {}", &config.log_level);
    info!("Worker count set to {}", &config.worker_count);
    info!("Server bind address set to '{}'", &config.bind);
    info!("Server port to '{}'", &config.port);
    info!(
        "Cookie store is {}",
        if config.enable_cookies {
            "enabled"
        } else {
            "disabled"
        }
    );
    info!(
        "Route configuration file path set to '{}'",
        &config.config_file
    );

    let http_client_config = Some(http_client_provider::ClientConfig {
        http_proxy: config.proxy_url,
        pass: config.proxy_auth_pass,
        user: config.proxy_auth_user,
        enable_cookies: config.enable_cookies,
    });

    http_client_provider::init(http_client_config)
        .map_err(|error| std::io::Error::new(ErrorKind::Other, error))?;

    let config_fd = fs::File::open(config.config_file)?;
    let path_configs: EndpointConfigFile = serde_yaml::from_reader(config_fd)
        .map_err(|err| std::io::Error::new(ErrorKind::Other, err))?;

    let endpoint_store: HashMap<EndpointKey, Arc<ProxyItem>> = HashMap::from_config(path_configs);

    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_header()
            .allow_any_method();

        let mut app = App::new().wrap(cors);

        for (key, _) in endpoint_store.iter() {
            if let Ok(method_type) = key.method.deref().try_into() {
                app = app.route(
                    &key.path,
                    match method_type {
                        HttpMethod::Get => web::get().to(handler),
                        HttpMethod::Post => web::post().to(handler),
                        HttpMethod::Put => web::put().to(handler),
                        HttpMethod::Delete => web::delete().to(handler),
                        HttpMethod::Head => web::head().to(handler),
                        HttpMethod::Patch => web::patch().to(handler),
                    },
                );
            } else {
                error!(
                    "Unsupported Http method '{}'. Ignoring Http mapping for '{}'",
                    key.method, key.path
                );
            }
        }

        app.app_data(web::Data::new(AppState {
            endpoint_map: endpoint_store.clone(),
        }))
    })
    .workers(config.worker_count)
    .bind((config.bind, config.port))?
    .run()
    .await
}

async fn handler(
    req: HttpRequest,
    data: web::Data<AppState>,
    payload: web::Payload,
) -> impl Responder {
    let key = EndpointKey::new(req.path(), &req.method().as_str().to_lowercase());
    if let Some(proxy_item) = data.endpoint_map.get(&key) {
        if let Ok(body_bytes) = payload.to_bytes().await {
            let response = proxy_item.execute(body_bytes).await;
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
                    error!(
                        "Proxy route request failed for '{}:{}'",
                        key.method, key.path
                    );
                    HttpResponse::BadRequest().body(err.to_string())
                }
            }
        } else {
            error!("Unable to parse body payload.");
            HttpResponse::BadRequest().finish()
        }
    } else {
        error!("Endpoint is configures but deallocated from endpoint store.");
        HttpResponse::InternalServerError().finish()
    }
}
