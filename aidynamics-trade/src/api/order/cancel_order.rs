use aidynamics_trade_utils::http::{EndPoint, HttpClient};

use std::fmt::Display;

use crate::{types::OrderVariety, Result};

/// Cancel order request
#[derive(Debug, Serialize,Clone)]
pub struct CancelOrderReq {
    /// Order variety
    pub variety: OrderVariety,
    /// Order ID
    #[serde(rename = "orderid")]
    pub order_id: String,
}

/// Cancel order response
#[derive(Debug, Deserialize,Serialize,Clone)]
pub struct CancelOrderRes {
    /// Order ID
    #[serde(rename = "orderid")]
    pub order_id: String,
    /// Unique order id
    #[serde(rename = "uniqueorderid")]
    pub unique_order_id: Option<String>,
}

impl CancelOrderReq {
    /// Returns a new instance for [`CancelOrderReq`]
    pub fn new<O>(order_variety: OrderVariety, order_id: O) -> Self
    where
        O: Display,
    {
        Self {
            variety: order_variety,
            order_id: order_id.to_string(),
        }
    }

    /// Sends the [`CancelOrderReq`] to the API and returns [`CancelOrderRes`]
    pub async fn send(&self, http: &HttpClient) -> Result<CancelOrderRes> {
        http.post(EndPoint::OrderModify, self)
            .await
            .and_then(|res| res.into_data())
    }
}
