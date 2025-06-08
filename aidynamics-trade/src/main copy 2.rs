#![allow(warnings)]
use core::task;
use std::collections::HashMap;
use std::ops::Sub;
use std::time::Instant;
use std::{string, thread, vec};

use aidynamics_trade::end_points;
use aidynamics_trade::market::LtpDataReq;
use aidynamics_trade::order::CancelOrderReq;
use aidynamics_trade::trade_handler::Trade;
use aidynamics_trade::types::ExchangeType;
use aidynamics_trade::websocket::angel_one_websocket::{
    AngelOneWebSocketClient, SubscriptionBuilder, SubscriptionExchange, SubscriptionMode,
};
use aidynamics_trade::websocket::check_status::check_server_status;
use aidynamics_trade::websocket::websocket_client::{Signal, WebSocketClient};
use aidynamics_trade::{
    market::{SearchScrip, SearchScripRes},
    order::{IndividualOrderStatus, OrderSetter, PlaceOrderReq, PlaceOrderRes},
    trade_engine::{Segment, Strategy, TradeEngine},
    types::{MarketDataExchange, ProductType, TransactionType},
    user::{self, Profile},
    // websocket::{self, websocket_client::Signal},
    // ws::{AngelOneWs, Message, SubscriptionBuilder, SubscriptionExchange, SubscriptionMode},
    Result,
    SmartConnect,
};
use aidynamics_trade_utils::ws::WsStream;
use dotenv::dotenv;
use firebase_rs::*;
use tokio::net::TcpStream;
use tokio::signal;
// use futures::channel::mpsc::Receiver;
use tokio::sync::mpsc::Receiver;

// use futures::channel::mpsc::Receiver;
use futures::{
    future::{join, join_all},
    SinkExt, StreamExt,
};
use reqwest::header::HeaderMap;
use reqwest::Client;
use serde::{de::value, Deserialize, Serialize};
use serde_json::json;
use serde_json::{de::Read, Value};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{self, sleep, Duration};
use tokio_tungstenite::tungstenite::client;
use tokio_tungstenite::tungstenite::protocol::Message as ws_message;

use aidynamics_trade::trade_engine::{self, TradeStatus};
use tokio::sync::mpsc;
use tokio::sync::mpsc::Sender;
use tokio_tungstenite::{connect_async, WebSocketStream};
use url::Url;

#[derive(Debug)]
pub enum WebSocketMessage {
    /// Type of message is String
    Text(String),
    // Add other message types if needed
    Json(Value),
}

/// NewMessage is struct to format websocket messages
#[derive(Serialize, Deserialize, Debug)]
pub struct NewMessage {
    /// Signal
    pub signal: Signal,
    /// Data received from
    // pub from: DataEndpoint,
    /// Data sent to
    // pub to: DataEndpoint,
    /// Data received
    pub data: Value, // Use `serde_json::Value` to handle any type
}

#[derive(Debug, Serialize, Deserialize)]
enum AuthStatus {
    #[serde(rename = "authorized")]
    Authorized,
    #[serde(rename = "unauthorized")]
    Unauthorized,
}

#[derive(Debug, Serialize, Deserialize)]
enum RestResponse {
    #[serde(rename = "success")]
    Success,
    #[serde(rename = "message")]
    message,
    #[serde(rename = "error")]
    error,
    #[serde(rename = "data")]
    data,
}

#[derive(Debug, Serialize, Deserialize)]
struct AuthResponse {
    #[serde(rename = "message")]
    message: String,
    #[serde(rename = "auth_status")]
    auth_status: AuthStatus,
}

async fn integrate_firebase(firebase_url: &str) {
    let firebase = Firebase::new(&firebase_url).unwrap().at("trades");
    tokio::spawn(async move {
        let stream = firebase.with_realtime_events().unwrap();
        stream
            .listen(
                |event_type, data| {
                    println!("Type: {:?} Data: {:?}", event_type, data);
                },
                |err| println!("{:?}", err),
                false,
            )
            .await;
    });

    let firebase_auth = Firebase::new(&firebase_url).unwrap().at("auth");
    tokio::spawn(async move {
        let stream = firebase_auth.with_realtime_events().unwrap();
        stream
            .listen(
                |event_type, data| {
                    println!("Type: {:?} Data: {:?}", event_type, data);
                },
                |err| println!("{:?}", err),
                false,
            )
            .await;
    });
}

async fn login_web_server(
    auth_token: &mut String,
    angelone_tokens: &mut Option<Value>,
    rust_password: &String,
    api_url: &String,
) -> Result<()> {
    // Create an HTTP client
    let client = Client::new();

    // Define the URL to send the POST request to
    let url = format!("{}{}", api_url, "/secure/login"); // Example URL, replace with your actual endpoint

    // Define the payload for the POST request
    let payload = json!({
        "name": "RUST",
        "password":rust_password
    });

    // Send the POST request with JSON payload
    let response = client.post(url).json(&payload).send().await?;

    // Check if the request was successful

    match response.error_for_status() {
        Ok(res) => {
            let response_json: Value = res.json().await?;
            println!("\n\nResponse: {}\n\n", response_json);

            if let Some(success) = response_json.get("success") {
                if success == true {
                    // Get the "message" object from the response
                    if let Some(message) = response_json.get("message") {
                        // Fetch "authToken" from "message"
                        if let Some(token) = message.get("authToken") {
                            if let Some(token_str) = token.as_str() {
                                // println!("\n\nACTUAL SERVER TOKEN {:?}\n\n", token_str);
                                // println!("\n\nCONVERTING TO STRING {:?}\n\n", token_str.to_string());
                                *auth_token = token_str.to_string(); // Update auth_token
                            }
                        } else {
                            println!("\n\nError fetching authToken");
                        }

                        // Fetch any additional tokens, e.g., "angelone_tokens"
                        if let Some(ao_token) = message.get("angeloneTokens") {
                            *angelone_tokens = Some(ao_token.clone());
                        }
                        // println!("\n\nAngelone : {:?}", angelone_tokens);
                        // println!("\n\nAssigned auth token : {:?}", auth_token);
                    }
                } else {
                    println!("Error: {:?}", response_json.get("message"));
                }
            }
        }
        Err(err) => {
            println!("ERROR : {:?}", err);
            if err.status() == Some(reqwest::StatusCode::UNAUTHORIZED) {
                println!("Invalid client_id or password!");
            }
        }
    }
Ok(())
}

#[tokio::main]
async fn main() {
    let api_key = dotenv::var("API_KEY").unwrap();
    let client_code = dotenv::var("CLIENT_CODE").unwrap();
    let pin = dotenv::var("PIN").unwrap();
    let otp_token = dotenv::var("OTP_TOKEN").unwrap();
    let firebase_url = dotenv::var("FIREBASE_URL").unwrap();
    let ws_url = dotenv::var("WS_URL").unwrap();
    let status_url = dotenv::var("STATUS_URL").unwrap();
    let rust_password = dotenv::var("NEW_RUST_PASSWORD").unwrap();
    let api_url = dotenv::var("API_URL").unwrap();
    // BANKNIFTY = 26009,NIFTY = 26000, FINNIFTY = 26037
    let INDICES_TOKENS = vec!["26009", "26000", "26037"];

    let api_key_clone = api_key.clone();
    let client_code_clone = client_code.clone();
    let pin_clone = pin.clone();
    let max_loss = 750.00;
    let max_price = 500.00;
    let max_trades = 2;
    let symbol: String = "BANKNIFTY".to_string();
    let strategy = Strategy::MomentumShift;
    let segment = Segment::EQ;
    let quantity = 15;
    let trade_sl_points: f64 = 50.00;
    let trigger_price: f64 = 0.0;
    let transaction_type: TransactionType = TransactionType::BUY;

    let mut auth_token = "".to_string();
    // let mut web_server_token = "";
    let mut angelone_tokens: Option<Value> = None;
    let mut jwt_token: String = "".to_string();
    let mut feed_token: String = "".to_string();
    let mut refresh_token = "".to_string();

    // println!("\n\nRUST PASSWORD IS ============ {:?}", &rust_password);

    login_web_server(
        &mut auth_token,
        &mut angelone_tokens,
        &rust_password,
        &api_url,
    )
    .await;

    if let Some(ao_tokens) = angelone_tokens {
        if let Some(jt) = ao_tokens.get("jwttoken") {
            if let Some(jt_str) = jt.as_str() {
                jwt_token = jt_str.to_string();
            }
            // println!("\n\n JWT TOKEN IS {:?} \n\n", jwt_token);
        }
        if let Some(rt) = ao_tokens.get("refreshtoken") {
            if let Some(rt_str) = rt.as_str() {
                refresh_token = rt_str.to_string();
            }
            // println!("\n\n REFRESH TOKEN IS {:?} \n\n", refresh_token);
        }
        if let Some(ft) = ao_tokens.get("feedtoken") {
            if let Some(ft_str) = ft.as_str() {
                feed_token = ft_str.to_string();
            }
            // println!("\n\n FEED TOKEN IS {:?} \n\n", feed_token);
        }
    }

    let jwt_token_clone = jwt_token.clone();
    let refresh_token_clone = refresh_token.clone();
    let feed_token_clone = feed_token.clone();
    let auth_token_clone = auth_token.clone();

    // Create an mpsc channel that will be used to send messages to the WebSocket server
    let (tx_ws, mut rx_ws) = mpsc::channel(100);

    let (tx_aows, mut rx_aows) = mpsc::channel(100);

    // Create an mpsc channel so you can send messages from the main thread and other tasks
    let (tx_main, mut rx_main) = tokio::sync::mpsc::channel::<Signal>(100);

    let tx_main_clone = tx_main.clone();

    loop {
        if check_server_status(&status_url.as_str(), Duration::from_secs(2)).await {
            break;
        } else {
            println!("\n\nServer is offline, retrying in 5 seconds...");
            sleep(Duration::from_secs(5)).await;
        }
    }

    // Start the WebSocket client and pass the `rx` receiver to receive messages
    WebSocketClient::connect(ws_url, rx_ws, tx_main_clone, auth_token).await;

    /// PROCESS ALL TRADE HANDLERS BEGIN
    async fn receive_messages(
        rx_main: &mut Receiver<Signal>,
        tx_main_clone: Sender<Signal>,
        tx_ws_clone: Sender<Signal>,
    ) {
        let mut trade_handlers: Vec<TradeEngine> = Vec::new();
        while let Some(msg) = rx_main.recv().await {
            // Process the message
            // println!("\n\nMessage is {:?}", msg);
            match msg {
                Signal::PriceFeed {
                    token,
                    ltp,
                    is_index,
                } => {
                    // println!("Ltp {:?} Token : {:?}", ltp, token);

                    for handler in trade_handlers.iter_mut() {
                        if is_index {
                            if handler.trigger_price != 0.0
                                && !handler.wait
                                && handler.trade_id == 0
                            {
                                if handler.transaction_type == TransactionType::BUY
                                    && ltp > handler.trigger_price
                                {
                                    handler.entry_req = Some(
                                        PlaceOrderReq::new(
                                            "KOKUYOCMLN-EQ",      // Symbol
                                            "16827",              // Token
                                            TransactionType::BUY, // Buy/Sell
                                        )
                                        .quantity(handler.quantity) // Set quantity
                                        .product_type(ProductType::IntraDay),
                                    ); // Set product type
                                    let tx_main_c = tx_main_clone.clone();
                                    handler.new_trade(tx_main_c).await;
                                } else if handler.transaction_type == TransactionType::SELL
                                    && ltp < handler.trigger_price
                                    && !handler.wait
                                    && handler.trade_id == 0
                                {
                                    handler.entry_req = Some(
                                        PlaceOrderReq::new(
                                            "KOKUYOCMLN-EQ",       // Symbol
                                            "16827",               // Token
                                            TransactionType::SELL, // Buy/Sell
                                        )
                                        .quantity(handler.quantity) // Set quantity
                                        .product_type(ProductType::IntraDay),
                                    ); // Set product type
                                    let tx_main_c = tx_main_clone.clone();
                                    handler.new_trade(tx_main_c).await;
                                }
                            }
                        } else {
                            if handler.symbol_token == token {
                                if handler.transaction_type == TransactionType::BUY
                                    && (handler.sl != 0.0
                                        && ltp <= (handler.trade_entry_price - handler.sl))
                                {
                                    // handler.squareoff_trade().await.unwrap();
                                    println!("\n\nHere square off trade");
                                } else if handler.transaction_type == TransactionType::SELL
                                    && (handler.sl != 0.0
                                        && ltp >= (handler.sl - handler.trade_entry_price))
                                {
                                    // handler.squareoff_trade().await.unwrap();
                                    println!("\n\nHere square off trade");
                                }
                            }
                        }
                    }
                }

                /// This is sent by other process which places order and after process returns
                Signal::OrderPlaced(order_data) => {
                    println!(
                        "\n\nError add trade handler... in main channel {:?}",
                        order_data
                    );
                }

                Signal::NewTradeEngine(engine) => {
                    println!("\n\nReceived signal on main channel");
                    let mut new_engine = engine.clone(); //TradeEngine::create_trade_engine(engine).await.unwrap();

                    if !trade_handlers
                        .iter()
                        .any(|engine| engine.trade_engine_id == new_engine.trade_engine_id)
                    {
                        if new_engine.trade_status == TradeStatus::Open {
                            let trans_type = if new_engine.transaction_type == TransactionType::BUY
                            {
                                TransactionType::SELL
                            } else {
                                TransactionType::BUY
                            };
                            println!("\nnew e = {:?}", new_engine);
                            let exit_req = PlaceOrderReq::new(
                                new_engine.trading_symbol.clone(),
                                new_engine.symbol_token.clone(),
                                trans_type,
                            )
                            .exchange(ExchangeType::NFO)
                            .quantity(new_engine.quantity)
                            .product_type(ProductType::IntraDay);
                            new_engine.exit_req = Some(exit_req);
                        }

                        println!(
                            "\n\nNew Engine trade Status = {:?} exit req {:?}",
                            new_engine.trade_status, new_engine.exit_req
                        );

                        trade_handlers.push(new_engine);
                    } else {
                        println!(
                            "TradeEngine with trade_engine_id {} already exists.",
                            new_engine.trade_engine_id
                        );
                    }
                    println!("\n\nNo of trade engine == {:?}", trade_handlers.len());
                }
                Signal::RemoveTradeEngine(trade_engine_id) => {
                    // Find and remove the trade engine with the matching ID from the vector
                    trade_handlers.retain(|engine| engine.trade_engine_id != trade_engine_id);
                }
                Signal::Disconnect(client_id) => {
                    for handler in trade_handlers.iter_mut() {
                        println!(
                            "Hander client id = {:?}, trade status = {:?}",
                            handler.client_id, handler.trade_status
                        );
                        if handler.client_id == client_id
                            && handler.trade_status == TradeStatus::Open
                        {
                            if let Some(req) = &handler.exit_req {
                                handler.squareoff_trade(tx_ws_clone.clone()).await;
                            }
                        }
                    }
                    // Retain only trade engines that do not match the `client_id`
                    trade_handlers.retain(|engine| engine.client_id == client_id);
                }
                _ => {
                    println!("\n\nOther signal received inside.... {:?}", msg);
                }
            }
        }
    }

    /// PROCESS TRADE HANDLERS END
    // Spawn another task to receive message from websocket
    let tx_main_cln = tx_main.clone();
    let tx_ws_clone = tx_ws.clone();
    tokio::spawn(async move {
        receive_messages(&mut rx_main, tx_main_cln, tx_ws_clone).await;
        sleep(Duration::from_secs(5));
    });

    let feedToken = feed_token_clone.clone();
    let clientCode = client_code.clone();
    let apiKey = api_key.clone();
    let wsUrl = format!(
        "{}clientCode={}&feedToken={}&apiKey={}",
        "wss://smartapisocket.angelone.in/smart-stream?", clientCode, feedToken, apiKey
    );
    let aows = AngelOneWebSocketClient::connect(wsUrl, rx_aows, tx_main.clone()).await;

    let tx_aows_clone = tx_aows.clone();

    let subscription = SubscriptionBuilder::new("abcde12345")
        .mode(SubscriptionMode::Ltp)
        // .subscribe(SubscriptionExchange::MCXFO, vec!["429116".into()])
        .subscribe(
            SubscriptionExchange::NSECM,
            vec!["26009".into(), "26000".into(), "26037".into()],
        )
        .subscribe(SubscriptionExchange::NSEFO, vec!["43125".into()])
        .build();

    tx_aows_clone
        .send(Signal::Subscribe(subscription))
        .await
        .unwrap();

    // LIVE PRICE FEED WEBSOCKET CONNECTION END

    loop {
        // std::thread::sleep(std::time::Duration::from_secs(1));
        // println!("Main function is still running...");
    }
}
