use reqwest::redirect::Policy;
use reqwest::Client;

pub struct HttpClientConfig {
  pub http_proxy: Option<String>,
  pub user: Option<String>,
  pub pass: Option<String>,
  pub enable_cookies: bool,
}

impl HttpClientConfig {
  pub fn to_client(self) -> Result<Client, reqwest::Error> {
    let HttpClientConfig {
      http_proxy,
      user,
      pass,
      enable_cookies,
    } = self;
    let mut client_builder = reqwest::ClientBuilder::new();

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

    let client = client_builder.redirect(Policy::limited(5)).build()?;

    Ok(client)
  }
}
