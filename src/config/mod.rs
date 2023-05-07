use anyhow::{anyhow, Ok, Result};
use async_trait::async_trait;
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue, CONTENT_TYPE},
    Client, Method, Response, Url,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::json;
use serde_json_lodash::get;
use serde_urlencoded;
use serde_yaml;
use std::collections::HashMap;
use std::str::FromStr;
use tokio::fs;

use crate::{cli::KeyValType, ExtraArgs};

mod geo_coding;
mod response_profile;

pub use geo_coding::{GeoCodingConfig, GeoCodingProfile};
pub use response_profile::ResponseProfile;

#[async_trait]
pub trait LoadConfig
where
    Self: Sized + DeserializeOwned + ValidateConfig,
{
    /// Load config from yaml file
    async fn load_yaml(path: &str) -> Result<Self> {
        let content = fs::read_to_string(path).await?;
        Self::from_yaml(&content)
    }

    /// Load config from yaml string
    fn from_yaml(content: &str) -> Result<Self> {
        let config: Self = serde_yaml::from_str(content)?;
        config.validate()?;
        Ok(config)
    }
}

pub trait ValidateConfig {
    fn validate(&self) -> Result<()>;
}

pub fn is_default<T: Default + PartialEq>(t: &T) -> bool {
    t == &T::default()
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RequestProfile {
    #[serde(with = "http_serde::method", default)]
    pub method: Method,

    pub url: Url,

    #[serde(skip_serializing_if = "empty_json_value", default)]
    pub params: Option<serde_json::Value>,

    #[serde(
        skip_serializing_if = "HeaderMap::is_empty",
        with = "http_serde::header_map",
        default
    )]
    pub headers: HeaderMap,

    #[serde(skip_serializing_if = "empty_json_value", default)]
    pub body: Option<serde_json::Value>,
}

pub struct ResponseExt(Response);

impl RequestProfile {
    pub async fn send(&self, args: &ExtraArgs) -> Result<ResponseExt> {
        let (headers, query, body) = self.generate(&args)?;

        let client = Client::new();
        let req = client
            .request(self.method.clone(), self.url.clone())
            .query(&query)
            .headers(headers)
            .body(body)
            .build()?;

        let res = client.execute(req).await?;

        Ok(ResponseExt(res))
    }

    fn generate(&self, args: &ExtraArgs) -> Result<(HeaderMap, serde_json::Value, String)> {
        let mut header = self.headers.clone();
        let mut query = self.params.clone().unwrap_or_else(|| json!({}));
        let mut body = self.body.clone().unwrap_or_else(|| json!({}));

        for (key_value_type, value) in args.clone().into_iter() {
            match key_value_type {
                KeyValType::Header => {
                    for (key, value) in &value {
                        header.insert(HeaderName::from_str(key)?, HeaderValue::from_str(value)?);
                    }

                    if !header.contains_key(CONTENT_TYPE) {
                        header.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
                    }
                }
                KeyValType::Query => {
                    for (key, value) in &value {
                        query[key] = json!(value);
                    }
                }
                KeyValType::Body => {
                    for (key, value) in &value {
                        body[key] = value.parse()?;
                    }
                }
            }
        }

        let content_type = get_content_type(&header);

        match content_type.as_deref() {
            Some("application/json") => {
                let body = serde_json::to_string(&body)?;
                Ok((header, query, body))
            }
            Some("application/x-www-form-urlencoded" | "multipart/form-data") => {
                let body = serde_urlencoded::to_string(&body)?;
                Ok((header, query, body))
            }
            _ => Err(anyhow::anyhow!("Unsupported content type!")),
        }
    }

    pub fn new(
        method: Method,
        url: Url,
        params: Option<serde_json::Value>,
        headers: HeaderMap,
        body: Option<serde_json::Value>,
    ) -> Self {
        Self {
            method,
            url,
            params,
            headers,
            body,
        }
    }

    pub fn get_url(&self, args: &ExtraArgs) -> Result<String> {
        let (_, params, _) = self.generate(args)?;
        // let params = self.params.clone();
        let mut url = self.url.clone();

        if !params.as_object().unwrap().is_empty() {
            let query = serde_qs::to_string(&params)?;
            url.set_query(Some(&query));
        }
        Ok(url.into())
    }
}

impl ValidateConfig for RequestProfile {
    fn validate(&self) -> Result<()> {
        if let Some(params) = self.params.as_ref() {
            if !params.is_object() {
                return Err(anyhow!(
                    "params must be an object but got\n{}",
                    serde_yaml::to_string(params)?
                ));
            }
        }

        if let Some(body) = self.body.as_ref() {
            if !body.is_object() {
                return Err(anyhow!(
                    "body must be an object but got\n{}",
                    serde_yaml::to_string(body)?
                ));
            }
        }

        Ok(())
    }
}

impl FromStr for RequestProfile {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        let mut url = Url::parse(s)?;

        let qs = url.query_pairs();
        let mut params = json!({});
        for (k, v) in qs {
            params[&*k] = v.parse()?;
        }

        url.set_query(None);

        Ok(RequestProfile::new(
            Method::GET,
            url,
            Some(params),
            HeaderMap::new(),
            None,
        ))
    }
}

impl ResponseExt {
    pub fn into_inner(self) -> Response {
        self.0
    }

    pub fn get_header_keys(&self) -> Vec<String> {
        self.0
            .headers()
            .keys()
            .map(|k| k.as_str().to_string())
            .collect()
    }

    pub async fn get_results(self, profile: &ResponseProfile) -> Result<serde_json::Value> {
        let res = self.0;
        let text = res.text().await?;

        Ok(get_json_value(&text, &profile.pick_results)?)
    }
}

pub fn get_content_type(headers: &HeaderMap) -> Option<String> {
    headers
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().unwrap().split(';').next().map(|s| s.to_string()))
}

pub fn get_status_text(res: &Response) -> Result<String> {
    Ok(format!("{:?} {}", res.version(), res.status()))
}

fn empty_json_value(v: &Option<serde_json::Value>) -> bool {
    v.as_ref().map_or(true, |v| {
        if v.is_object() {
            if let Some(obj) = v.as_object() {
                return obj.is_empty();
            }
        }

        true
    })
}

fn get_json_value(text: &str, pick_results: &HashMap<String, String>) -> Result<serde_json::Value> {
    let obj: HashMap<String, serde_json::Value> = serde_json::from_str(text)?;

    let res = pick_results
        .into_iter()
        .map(move |(path, name)| (name, get!(json!(obj), json!(path))))
        .collect::<serde_json::Value>();

    Ok(res)
}
