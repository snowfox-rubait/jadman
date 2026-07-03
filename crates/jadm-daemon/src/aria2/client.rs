use crate::aria2::types::{Aria2Request, Aria2Response, Aria2Status};
use reqwest::Client;
use serde_json::json;
use anyhow::{Result, anyhow};

#[cfg_attr(test, mockall::automock)]
#[async_trait::async_trait]
pub trait Aria2ClientTrait: Send + Sync {
    async fn add_uri(&self, uri: &str, options: serde_json::Value) -> Result<String>;
    async fn tell_status(&self, gid: &str) -> Result<Aria2Status>;
    async fn pause(&self, gid: &str) -> Result<String>;
    async fn unpause(&self, gid: &str) -> Result<String>;
    async fn remove(&self, gid: &str) -> Result<String>;
    async fn change_global_option(&self, options: serde_json::Value) -> Result<String>;
}

pub struct Aria2Client {
    client: Client,
    url: String,
    secret: Option<String>,
}

impl Aria2Client {
    pub fn new(url: String, secret: Option<String>) -> Self {
        Self {
            client: Client::new(),
            url,
            secret,
        }
    }

    async fn call<T: for<'de> serde::Deserialize<'de>>(&self, method: &str, params: Vec<serde_json::Value>) -> Result<T> {
        let mut final_params = Vec::new();
        if let Some(ref secret) = self.secret {
            final_params.push(json!(format!("token:{}", secret)));
        }
        for p in params {
            final_params.push(p);
        }

        let request = Aria2Request {
            jsonrpc: "2.0".to_string(),
            id: "jadm".to_string(),
            method: format!("aria2.{}", method),
            params: final_params,
        };

        let response: Aria2Response<T> = self.client.post(&self.url)
            .json(&request)
            .send()
            .await?
            .json()
            .await?;

        if let Some(error) = response.error {
            return Err(anyhow!("Aria2 error {}: {}", error.code, error.message));
        }

        response.result.ok_or_else(|| anyhow!("Empty result from aria2"))
    }
}

#[async_trait::async_trait]
impl Aria2ClientTrait for Aria2Client {
    async fn add_uri(&self, uri: &str, options: serde_json::Value) -> Result<String> {
        self.call("addUri", vec![json!([uri]), options]).await
    }

    async fn tell_status(&self, gid: &str) -> Result<Aria2Status> {
        self.call("tellStatus", vec![json!(gid)]).await
    }

    async fn pause(&self, gid: &str) -> Result<String> {
        self.call("pause", vec![json!(gid)]).await
    }

    async fn unpause(&self, gid: &str) -> Result<String> {
        self.call("unpause", vec![json!(gid)]).await
    }

    async fn remove(&self, gid: &str) -> Result<String> {
        self.call("remove", vec![json!(gid)]).await
    }

    async fn change_global_option(&self, options: serde_json::Value) -> Result<String> {
        self.call("changeGlobalOption", vec![options]).await
    }
}
