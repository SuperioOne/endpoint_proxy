use reqwest::redirect::Policy;
use reqwest::Client;

pub struct ClientConfig {
    pub http_proxy: Option<String>,
    pub user: Option<String>,
    pub pass: Option<String>,
    pub enable_cookies: bool,
}

static mut HTTP_CLIENT: Option<Client> = None;

pub fn init(config: Option<ClientConfig>) -> Result<(), reqwest::Error> {
    let mut client_builder = reqwest::ClientBuilder::new();

    if let Some(client_config) = config {
        let ClientConfig {
            http_proxy,
            user,
            pass,
            enable_cookies,
        } = client_config;

        if let Some(proxy_url) = http_proxy {
            let mut proxy = reqwest::Proxy::all(proxy_url)?;

            if let (Some(user_name), Some(password)) = (user, pass) {
                proxy = proxy.basic_auth(&user_name, &password);
            }

            client_builder = client_builder.proxy(proxy);
        }

        if enable_cookies {
            client_builder = client_builder.cookie_store(true);
        }
    }
    let client = client_builder.redirect(Policy::limited(5)).build()?;
    unsafe {
        HTTP_CLIENT = Some(client);
    }

    Ok(())
}

pub fn get_client() -> &'static Client {
    unsafe {
        if let Some(client) = &HTTP_CLIENT {
            client
        } else {
            if let Err(_) = init(Option::None) {
                panic!("Unable to build reqwest HTTP client.")
            }

            if let Some(client) = &HTTP_CLIENT {
                client
            } else {
                panic!("Client initialized but still empty.")
            }
        }
    }
}
