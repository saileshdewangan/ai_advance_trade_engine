
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use serde_json::json;
use std::{error::Error, sync::Arc};

use crate::{order::PlaceOrderRes, server_utils::end_points};

use super::server_http::ServerHttp;

// use crate::order::PlaceOrderRes;

/// Order placed is entry or exit
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum OrderPlacedType {
    /// Order is entry type
    #[serde(rename = "entry")]
    Entry,
    /// Order is exit type
    #[serde(rename = "exit")]
    Exit,
}

pub async fn update_orders_response(
    order_place_type: OrderPlacedType,
    orders: Vec<Arc<PlaceOrderRes>>,
server_http:ServerHttp) -> Result<(), Box<dyn Error>> {
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    // headers.insert("Authorization", HeaderValue::from_static("Bearer your-token"));

    // Convert each Arc<PlaceOrderRes> into a serializable JSON object
    let serialized_orders: Vec<_> = orders.iter().map(|order| {
        json!({
            "script": order.script,
            "orderid": order.order_id,
            "uniqueorderid": order.unique_order_id,
        })
    }).collect();

    // Construct the body with the order type and the serialized orders
    let json_body = json!({
        "ordertype": order_place_type,
        "orders": serialized_orders,
    });

    let response = server_http.post(end_points::UPDATES.to_string(),json_body).await?;
    let body = response.text().await?;
    println!("Response: {}", body);

    Ok(())
}


// UPDATING TRADE PLACED TO DATABASE SERVER BEGIN

// UPDATING TRADE PLACED TO DATABASE SERVER END