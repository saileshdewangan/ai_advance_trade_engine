use reqwest::{header, ClientBuilder, Response};
use serde_json::Value;
use std::sync::Arc;

use crate::{redis_utils::Signal, trade_engine::TradeRes};
// use tokio::task;

#[derive(Debug, Clone)]
pub struct ServerHttp {
    pub client: Arc<reqwest::Client>,
    base_url: String, // Private field to store base URL
}

impl ServerHttp {
    pub fn new() -> ServerHttp {
        let base_url = dotenv::var("API_URL").expect("API_URL must be set");
        let webserver_auth_token = dotenv::var("WEBSERVER_AUTH_TOKEN").unwrap();
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            header::HeaderValue::from_str(&format!("Bearer {}", webserver_auth_token)).unwrap(),
        );

        let client = ClientBuilder::new()
            .default_headers(headers) // Set the default headers, including Authorization
            .timeout(std::time::Duration::from_secs(10)) // Set a timeout
            .build()
            .expect("Failed to build client");

        ServerHttp {
            client: Arc::new(client),
            base_url,
        }
    }

    pub fn url(&self) -> &str {
        &self.base_url
    }

    pub fn new_with_auth(auth_token: String) -> ServerHttp {
        let base_url = dotenv::var("API_URL").expect("API_URL must be set");
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            header::HeaderValue::from_str(&format!("Bearer {}", auth_token)).unwrap(),
        );

        let client = ClientBuilder::new()
            .default_headers(headers) // Set the default headers, including Authorization
            .timeout(std::time::Duration::from_secs(10)) // Set a timeout
            .build()
            .expect("Failed to build client");

        ServerHttp {
            client: Arc::new(client),
            base_url,
        }
    }

    pub async fn update_trade_data(&self, payload: Value) -> Result<TradeRes, Box<dyn std::error::Error>> {
        let url = format!("{}/secure/server/update-executed-data", self.url());
        let resp = self.client.post(url).json(&payload).send().await?;
        let json_string = resp.text().await?;
        let json_value: Value = serde_json::from_str(&json_string)?;

        let data = json_value.get("data").ok_or_else(|| "Data field not found".to_string())?;
        let signal = data.get("signal").ok_or_else(|| "Signal field not found".to_string())?;

        match serde_json::from_value::<Signal>(signal.clone())? {
            Signal::OrderPlaced(trade_res) => Ok(trade_res),
            _ => Err("Expected OrderPlaced signal".into()), // Convert to Box<dyn Error>
        }
    }

    pub async fn execute_demo_trade(&self, payload: Value) -> Result<TradeRes, Box<dyn std::error::Error>> {
        let url = format!("{}/secure/server/executed-demo-trade", self.url());
        let resp = self.client.post(url).json(&payload).send().await?;
        let json_string = resp.text().await?;
        let json_value: Value = serde_json::from_str(&json_string)?;

        let data = json_value.get("data").ok_or_else(|| "Data field not found".to_string())?;
        let signal = data.get("signal").ok_or_else(|| "Signal field not found".to_string())?;

        match serde_json::from_value::<Signal>(signal.clone())? {
            Signal::OrderPlaced(trade_res) => Ok(trade_res),
            _ => Err("Expected OrderPlaced signal".into()), // Convert to Box<dyn Error>
        }
    }


    pub async fn get(&self, url: String) -> Result<Response, reqwest::Error> {
        // let res = self.client.get(url).send().await?.text().await?;
        let res = self
            .client
            .get(format!("{}/{}", self.base_url, url))
            .send()
            .await?;
        Ok(res)
    }

    pub async fn post(
        &self,
        url: String,
        body: serde_json::Value,
    ) -> Result<Response, reqwest::Error> {
        let res = self
            .client
            .post(format!("{}/{}", self.base_url, url))
            .json(&body)
            .send()
            .await?;
        Ok(res)
    }
}
