use std::collections::HashMap;
use std::sync::Arc;

use log::info;

use crate::proxy_item::ProxyItem;
use crate::{EndpointConfigFile, ConfigProxyConfig, HttpMethod};

pub trait EndpointMap {
    fn from_config(config: EndpointConfigFile) -> Self;
}

#[derive(PartialEq, Hash, Eq, Clone)]
pub struct EndpointKey {
    pub path: Arc<str>,
    pub method: Arc<str>,
}

impl EndpointKey {
    pub fn new(path: &str, method: &str) -> EndpointKey {
        EndpointKey {
            method: Arc::from(method),
            path: Arc::from(path),
        }
    }
}

impl EndpointMap for HashMap<EndpointKey, Arc<ProxyItem>> {
    fn from_config(config: EndpointConfigFile) -> Self {
        let mut endpoint_store: HashMap<EndpointKey, Arc<ProxyItem>> = HashMap::new();
        for config_item in config.proxy_urls.into_iter() {
            let ConfigProxyConfig {
                query,
                url,
                path,
                headers,
                method,
                target_method,
                default_body,
            } = config_item;

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

            if let Some(http_target_method) = target_method.or_else(|| method.clone()) {
                proxy_item = proxy_item.set_method(http_target_method);
            }

            if let Some(default_body) = default_body {
                proxy_item = proxy_item.set_default_body(default_body.as_bytes());
            }

            let method_key = if let Some(method) = &method {
                method.to_string()
            } else {
                HttpMethod::Get.to_string()
            };

            let value: Arc<ProxyItem> = Arc::from(proxy_item);
            let key = EndpointKey::new(path.as_str(), &method_key);
            endpoint_store.insert(key, value);
            info!("New endpoint created at '{}:{}'.", &method_key, &path);
        }

        endpoint_store
    }
}
