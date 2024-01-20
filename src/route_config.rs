use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io::ErrorKind;

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Copy, Hash, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum HttpMethod {
  #[default]
  Get,
  Post,
  Put,
  Delete,
  Head,
  Patch,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct NameValuePair {
  pub name: String,
  pub value: String,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct RouteConfig {
  pub path: String,
  pub url: String,
  pub query: Option<Vec<NameValuePair>>,
  pub headers: Option<Vec<NameValuePair>>,
  pub method: Option<HttpMethod>,
  pub target_method: Option<HttpMethod>,
  pub default_body: Option<String>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct EndpointConfigFile {
  pub proxy_urls: Vec<RouteConfig>,
}

impl EndpointConfigFile {
  pub fn load_from_file(file: &File) -> Result<EndpointConfigFile, std::io::Error> {
    let path_configs: EndpointConfigFile =
      serde_yaml::from_reader(file).map_err(|err| std::io::Error::new(ErrorKind::Other, err))?;

    Ok(path_configs)
  }
}

impl TryFrom<&str> for HttpMethod {
  type Error = ();

  fn try_from(value: &str) -> Result<Self, Self::Error> {
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

impl Display for HttpMethod {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      HttpMethod::Get => f.write_str("get"),
      HttpMethod::Post => f.write_str("post"),
      HttpMethod::Put => f.write_str("put"),
      HttpMethod::Delete => f.write_str("delete"),
      HttpMethod::Head => f.write_str("head"),
      HttpMethod::Patch => f.write_str("patch"),
    }
  }
}
