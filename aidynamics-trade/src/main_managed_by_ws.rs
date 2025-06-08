#![allow(warnings)]
use core::task;
use std::collections::HashMap;
use std::ops::Sub;
use std::time::Instant;
use std::{clone, string, thread, vec};
// use std::{array, f64::consts::E, fmt::write, intrinsics::mir::place, iter, str::FromStr};

use aidynamics_trade::market::LtpDataReq;
use aidynamics_trade::order::CancelOrderReq;
use aidynamics_trade::trade_handler::Trade;
use aidynamics_trade::types::ExchangeType;
use aidynamics_trade::websocket::angel_one_websocket::{
    AngelOneWebSocketClient, SubscriptionBuilder, SubscriptionExchange, SubscriptionMode,
};
use aidynamics_trade::websocket::check_status::check_server_status;
use aidynamics_trade::websocket::websocket_client::{Signal, WebSocketClient};
use aidynamics_trade::{end_points, trade_handler};
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

// use websocket::check_status::check_server_status;
// use websocket::websocket_client::{
//     connect_to_websocket, send_message as message_sender, DataEndpoint, NewMessage,
//     WebSocketMessage,
// };

// use websocket_client::WebSocketClient;
use aidynamics_trade::trade_engine::{self, EngineStatus, TradeStatus};
use tokio::sync::mpsc;
use tokio::sync::mpsc::Sender;
use tokio_tungstenite::{connect_async, WebSocketStream};

// use futures_util::{SinkExt, StreamExt};
// use tokio::sync::mpsc;
// use tokio_tungstenite::tungstenite::protocol::Message;
// use tokio_tungstenite::connect_async;
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

/// Define the `Signal` enum with variants for each type of message
// #[derive(Debug, Serialize, Deserialize)]
// pub enum Signal {
//     /// Add new client with configuration
//     #[serde(rename = "new_trade_engine")]
//     NewTradeEngine,

//     /// This will be forwarded to channel to add to handler vec
//     #[serde(rename = "add_trade_engine")]
//     AddNewTradeEngine(TradeEngineConfig),

//     /// General message
//     #[serde(rename = "message")]
//     Message,

//     /// Connection established
//     #[serde(rename = "connection")]
//     Connection,

//     /// Live Price feed
//     #[serde(rename = "pricefeed")]
//     PriceFeed {
//         token: String,
//         ltp: f64,
//         is_index: bool,
//     },

//     /// Strategy matched
//     #[serde(rename = "strategy_matched")]
//     StrategyMatched,

//     /// Authorization
//     #[serde(rename = "auth")]
//     Auth,

//     /// Order Placed all other API are called and completed
//     #[serde(rename = "order_placed")]
//     OrderPlaced(NewTradeRes),
// }

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

// async fn add_trade_engine(config: TradeEngineConfig, tx_ws: Sender<Signal>) -> Result<TradeEngine> {
//     println!("Adding trading engine ...");
//     // tx_ws
//     //     .send(Signal::AddNewTradeEngine(config.clone()))
//     //     .await
//     //     .unwrap();
//     // let new_engine = TradeEngine::new(config).await.unwrap();
//     let new_engine =
//     // println!("Client code = {:?}", new_handler.client.client_code);
//     Ok(new_engine)
// }

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

            // if let Some(status) = response_json.get("status") {
            //     if status == "success" {
            //         if let Some(token) = response_json.get("authToken") {
            //             if let Some(token_str) = token.as_str() {
            //                 println!("\n\nACTUAL SERVER TOKEN {:?}\n\n", token_str);
            //                 println!("\n\nCONVERTING TO STRING {:?}\n\n", token_str.to_string());
            //                 *auth_token = token_str.to_string();
            //             }
            //         } else {
            //             println!("Error fetching tokens");
            //         }
            //         if let Some(ao_token) = response_json.get("angelone_tokens") {
            //             *angelone_tokens = Some(ao_token.clone());
            //         }
            //     } else {
            //         println!("Error {:?}", Some(response_json.get("message")));
            //     }
            // }

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

    // if response.status().is_success() {

    // } else {
    //     eprintln!("Failed to send POST request. Status: {}", response.status());
    // }
    Ok(())
}

// struct WebSocketClient;

// impl WebSocketClient {
//     // Initialize and spawn the WebSocket client task
//     async fn connect(
//         ws_url: String,
//         mut rx: mpsc::Receiver<Signal>,
//         tx: mpsc::Sender<Signal>,
//         auth_token: String,
//     ) {
//         let tx_clone = tx.clone();

//         // Spawn the WebSocket client task
//         tokio::task::spawn(async move {
//             // Connect to the WebSocket server
//             let (ws_stream, _) = connect_async(ws_url).await.expect("Failed to connect");

//             println!("WebSocket connection established");

//             // Split the WebSocket stream into a sender (sink) and receiver (stream)
//             let (write, mut read) = ws_stream.split();

//             println!("AUTH TOKEN IS = {:?}", auth_token);

//             // Send the Auth message immediately after the connection is established
//             let auth_message = NewMessage {
//                 signal: Signal::Auth,
//                 data: serde_json::json!({
//                     "token": auth_token,  // Send your authentication token
//                 }),
//             };

//             // Serialize the message and send it
//             let serialized_message = serde_json::to_string(&auth_message).unwrap();
//             let mut write = Arc::new(Mutex::new(write));

//             {
//                 let mut write_locked = write.lock().await;
//                 if let Err(e) = write_locked
//                     .send(ws_message::Text(serialized_message))
//                     .await
//                 {
//                     println!("Error sending Auth message: {:?}", e);
//                 }
//             }

//             // Task to handle incoming WebSocket messages
//             let read_task = tokio::spawn({
//                 let write_clone = write.clone(); // Clone `write` for later use

//                 async move {
//                     while let Some(msg) = read.next().await {
//                         match msg {
//                             Ok(message) => {
//                                 match message {
//                                     ws_message::Text(text) => {
//                                         println!("WS MESSAGE RECEIVED : {:?}", text);
//                                         match serde_json::from_str::<NewMessage>(&text) {
//                                             Ok(msg) => {
//                                                 // Handle each signal
//                                                 match msg.signal {
//                                                     Signal::Auth => {
//                                                         println!("Auth data : {:?}", msg.data);
//                                                         let trade_engine_config =
//                                                             TradeEngineConfig {
//                                                                 trade_engine_id: 1,
//                                                                 client_id: "930317".to_string(),
//                                                                 max_loss: 500.0,
//                                                                 max_price: 500.0,
//                                                                 max_trades: 2,
//                                                                 symbol: "".to_string(),
//                                                                 strategy: Strategy::DynamicChannel,
//                                                                 segment: Segment::EQ,
//                                                                 sl: 50.0,
//                                                                 trigger_price: 0.0,
//                                                                 quantity: 15,
//                                                                 transaction_type:
//                                                                     TransactionType::BUY,
//                                                                 auth_token: "".to_string(),
//                                                             };
//                                                         tx_clone
//                                                             .send(Signal::AddNewTradeEngine(
//                                                                 trade_engine_config,
//                                                             ))
//                                                             .await;
//                                                     }
//                                                     Signal::NewTradeEngine => {
//                                                         println!(
//                                                             "NEW TRADE HANDLER DATA : {:?}",
//                                                             msg.data
//                                                         );
//                                                         let config = msg.data;
//                                                         let trade_engine_config =
//                                                             TradeEngineConfig {
//                                                                 trade_engine_id: 1,
//                                                                 client_id: "930317".to_string(),
//                                                                 max_loss: 500.0,
//                                                                 max_price: 500.0,
//                                                                 max_trades: 2,
//                                                                 symbol: "".to_string(),
//                                                                 strategy: Strategy::DynamicChannel,
//                                                                 segment: Segment::EQ,
//                                                                 sl: 50.0,
//                                                                 trigger_price: 0.0,
//                                                                 quantity: 15,
//                                                                 transaction_type:
//                                                                     TransactionType::BUY,
//                                                                 auth_token: "".to_string(),
//                                                             };
//                                                         tx_clone
//                                                             .send(Signal::AddNewTradeEngine(
//                                                                 trade_engine_config,
//                                                             ))
//                                                             .await;
//                                                     }
//                                                     Signal::StrategyMatched => {
//                                                         println!("Strategy matched.");
//                                                     }
//                                                     _ => {
//                                                         println!("Other signal received....");
//                                                     }
//                                                 }
//                                             }
//                                             Err(e) => {
//                                                 eprintln!("Failed to deserialize message: {}", e);
//                                             }
//                                         }
//                                     }
//                                     ws_message::Pong(payload) => {
//                                         println!("Pong Payload = {:?}", payload);
//                                     }
//                                     _ => {
//                                         println!("Other message type received");
//                                     }
//                                 }
//                             }
//                             Err(e) => {
//                                 eprintln!("Error receiving message: {}", e);
//                             }
//                         }
//                     }
//                 }
//             });

//             // Task to listen for messages from the main thread and send them to the WebSocket
//             let write_clone = write.clone();
//             tokio::spawn(async move {
//                 while let Some(message) = rx.recv().await {
//                     let mut write_locked = write_clone.lock().await;
//                     let msg_to_send = ws_message::Text(serde_json::to_string(&message).unwrap());
//                     if let Err(e) = write_locked.send(msg_to_send).await {
//                         eprintln!("Failed to send message: {}", e);
//                     }
//                 }
//             });

//             // Task to send ping messages periodically every 30 seconds
//             tokio::spawn({
//                 let write_clone = write.clone();

//                 async move {
//                     loop {
//                         sleep(Duration::from_secs(30)).await;
//                         let mut write_locked = write_clone.lock().await;
//                         if let Err(e) = write_locked.send(ws_message::Ping(vec![])).await {
//                             eprintln!("Error sending ping: {:?}", e);
//                             break;
//                         }
//                         println!("Sent ping");
//                     }
//                 }
//             });

//             // Wait for the read task to finish
//             if let Err(e) = read_task.await {
//                 eprintln!("\n\nRead task failed: {}", e);
//             }

//             println!("\n\nWebSocket connection closed.");
//         });
//     }
// }

async fn handle_buy_signal(handler: &mut TradeEngine, ltp: f32, tx_ws: Sender<Signal>) {
    if handler.transaction_type == TransactionType::BUY && ltp > handler.trigger_price {
        // handler.entry_req = Some(
        //     PlaceOrderReq::new("KOKUYOCMLN-EQ", "16827", TransactionType::BUY)
        //         .quantity(handler.quantity)
        //         .product_type(ProductType::IntraDay),
        // );
        handler.execute_trade(tx_ws).await;
    }
}

async fn handle_sell_signal(handler: &mut TradeEngine, ltp: f32, tx_ws: Sender<Signal>) {
    if handler.transaction_type == TransactionType::SELL && ltp < handler.trigger_price {
        // handler.entry_req = Some(
        //     PlaceOrderReq::new("KOKUYOCMLN-EQ", "16827", TransactionType::SELL)
        //         .quantity(handler.quantity)
        //         .product_type(ProductType::IntraDay),
        // );
        handler.execute_trade(tx_ws.clone()).await;
    }
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

    let mut ws_server_ready = Arc::new(Mutex::new(false));
    let ws_ready_clone = Arc::clone(&ws_server_ready);

    let mut auth_token = "".to_string();
    // let mut web_server_token = "";
    let mut angelone_tokens: Option<Value> = None;
    let mut jwt_token: String = "".to_string();
    let mut feed_token: String = "".to_string();
    let mut refresh_token = "".to_string();

    loop {
        if check_server_status(&status_url.as_str(), Duration::from_secs(2)).await {
            break;
        } else {
            println!("\n\nServer is offline, retrying in 5 seconds...");
            sleep(Duration::from_secs(5)).await;
        }
    }

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

    // loop {
    //     if check_server_status(&status_url.as_str(), Duration::from_secs(2)).await {
    //         break;
    //     } else {
    //         println!("\n\nServer is offline, retrying in 5 seconds...");
    //         sleep(Duration::from_secs(5)).await;
    //     }
    // }

    // Start the WebSocket client and pass the `rx` receiver to receive messages
    WebSocketClient::connect(ws_url.clone(), rx_ws, tx_main_clone.clone(), auth_token).await;

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
                                && !handler.pause
                            {
                                match handler.transaction_type {
                                    TransactionType::BUY => {
                                        handle_buy_signal(handler, ltp, tx_ws_clone.clone()).await
                                    }
                                    TransactionType::SELL => {
                                        handle_sell_signal(handler, ltp, tx_ws_clone.clone()).await
                                    }
                                    _ => {}
                                }

                                // if handler.transaction_type == TransactionType::BUY
                                //     && ltp > handler.trigger_price
                                // {
                                //     handler.entry_req = Some(
                                //         PlaceOrderReq::new(
                                //             "KOKUYOCMLN-EQ",      // Symbol
                                //             "16827",              // Token
                                //             TransactionType::BUY, // Buy/Sell
                                //         )
                                //         .quantity(handler.quantity) // Set quantity
                                //         .product_type(ProductType::IntraDay),
                                //     ); // Set product type
                                //     let tx_main_c = tx_main_clone.clone();
                                //     handler.new_trade(tx_main_c).await;
                                // } else if handler.transaction_type == TransactionType::SELL
                                //     && ltp < handler.trigger_price
                                //     && !handler.wait
                                // {
                                //     handler.entry_req = Some(
                                //         PlaceOrderReq::new(
                                //             "KOKUYOCMLN-EQ",       // Symbol
                                //             "16827",               // Token
                                //             TransactionType::SELL, // Buy/Sell
                                //         )
                                //         .quantity(handler.quantity) // Set quantity
                                //         .product_type(ProductType::IntraDay),
                                //     ); // Set product type
                                //     let tx_main_c = tx_main_clone.clone();
                                //     handler.new_trade(tx_main_c).await;
                                // }
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
                Signal::NewTrade(engine) => {
                    println!("\n\nMain channel : NewTrade");
                    if let Some(existing_engine) = trade_handlers
                        .iter_mut()
                        .find(|existing| existing.trade_engine_id == engine.trade_engine_id)
                    {
                        existing_engine.trade_status = TradeStatus::Open;
                        existing_engine.symbol_token = engine.symbol_token;
                        existing_engine.trigger_price = engine.trigger_price;
                        existing_engine.trading_symbol = engine.trading_symbol;

                        // Call the method to prepare entry
                        existing_engine.prepare_entry_req();
                    } else {
                        println!(
                            "No TradeEngine with corresponding trade_engine_id. {:?}",
                            engine.trade_engine_id
                        );
                    }
                }
                Signal::NewTradeEngine(engine) => {
                    // Signal::AddTradeEngine(engine) => {
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
                            .exchange(new_engine.exchange_type)
                            .quantity(new_engine.quantity)
                            .product_type(ProductType::IntraDay);
                            new_engine.exit_req = Some(exit_req);
                        }

                        println!(
                            "\n\nNew Engine trade Status = {:?} exit req {:?}",
                            new_engine.trade_status, new_engine.exit_req
                        );

                        trade_handlers.push(new_engine);
                        println!(
                            "\n\nAdded : No of trade engines = {:?}",
                            trade_handlers.len()
                        );
                    } else {
                        println!(
                            "TradeEngine with trade_engine_id {} already exists.",
                            new_engine.trade_engine_id
                        );
                    }

                    // trade_handlers.push(new_engine);
                    // match add_trade_engine(config, tx_ws_clone.clone()).await {
                    //     Ok(trade_handler) => {
                    //         trade_handlers.push(trade_handler);
                    //     }
                    //     Err(e) => {
                    //         println!("\n\nError add trade handler... in main channel {:?}", e);
                    //     }
                    // }
                    println!("\n\nNo of trade engine == {:?}", trade_handlers.len());
                }
                Signal::RemoveTradeEngine(trade_engine_id) => {
                    // Find and remove the trade engine with the matching ID from the vector
                    trade_handlers.retain(|engine| engine.trade_engine_id != trade_engine_id);
                    println!(
                        "\n\nRemoved : No of trades engines = {:?}",
                        trade_handlers.len()
                    );
                }
                Signal::TradeEngineExist { client_id, status } => {
                    if let Some(trd_engine) = trade_handlers
                        .iter()
                        .find(|handler| handler.client_id == client_id)
                    {
                        tx_ws_clone
                            .send(Signal::TradeEngineExist {
                                client_id,
                                status: true,
                            })
                            .await;
                    } else {
                        tx_ws_clone
                            .send(Signal::TradeEngineExist {
                                client_id,
                                status: false,
                            })
                            .await;
                    }
                }
                Signal::UpdateTradeEngine {
                    trade_engine_id,
                    client_id,
                    config,
                } => {
                    if let Some(handler) = trade_handlers.iter_mut().find(|handler| {
                        handler.client_id == client_id
                            && handler.trade_status == TradeStatus::Closed
                            && handler.trade_engine_id == trade_engine_id // Check for matching trade_id
                    }) {
                        handler.max_loss = config.max_loss;
                        handler.max_price = config.max_price;
                        handler.max_trades = config.max_trades;
                        handler.quantity = config.quantity;
                        handler.sl = config.sl;
                        handler.strategy = config.strategy;
                        handler.target = config.target;
                        handler.trailing_sl = config.trailing_sl;
                        handler.transaction_type = config.transaction_type;
                    }
                }
                Signal::SquareOffTradeReq {
                    client_id,
                    trade_id,
                } => {
                    if let Some(handler) = trade_handlers.iter_mut().find(|handler| {
                        handler.client_id == client_id
                            && handler.trade_status == TradeStatus::Open
                            && handler.trade_id == trade_id // Check for matching trade_id
                    }) {
                        if let Some(req) = &handler.exit_req {
                            // handler.squareoff_trade(tx_ws_clone.clone()).await;

                            println!("Order Req = {:?}", req);

                            tx_ws_clone
                                .send(Signal::SquaredOff {
                                    trade_id: handler.trade_id,
                                    order_req: req.clone(),
                                    client_id: handler.client_id,
                                })
                                .await;
                        }
                    } else {
                        println!(
                            "Handler not found for client_id = {:?} and trade_id = {:?}",
                            client_id, trade_id
                        );
                        tx_ws_clone
                            .send(Signal::SquareOffReject {
                                trade_id,
                                error: "No trade found".to_string(),
                                message: "No trade found for this client".to_string(),
                            })
                            .await;
                    }
                }
                Signal::Disconnect(client_id) => {
                    for handler in trade_handlers.iter_mut() {
                        println!(
                            "\nHander client id = {:?}, trade status = {:?}",
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
                    println!("\nHandlers count -> {:?}", trade_handlers.len());
                }
                Signal::WsConnectionClosed => {
                    // let mut ws_ready = ws_ready_clone.lock().unwrap();
                    // *ws_ready = false;
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
        // sleep(Duration::from_secs(5));
    });

    // let indice_token = vec!["26009".to_string()];
    // let ao_ws = AngelOneWs::new(client_code, feed_token);

    // let mut last_price: f64 = 0.0;
    // let price_change_threshold: f64 = 10.0; // Only emit if the price changes by 10 or more

    // let mut ws_stream = ao_ws.stream::<Message>().await.unwrap();

    // let subscription_message = SubscriptionBuilder::new("abcde12345")
    //     // .mode(SubscriptionMode::Quote)
    //     .mode(SubscriptionMode::Ltp)
    //     .subscribe(SubscriptionExchange::NSECM, vec!["26009"])
    //     // .subscribe(
    //     //     SubscriptionExchange::NSEFO,
    //     //     vec!["48436", "48444"],
    //     // )
    //     //  429116" SILVERMIC24NOVFUT
    //     // .subscribe(SubscriptionExchange::MCXFO, vec!["429116"])
    //     // .subscribe(
    //     //     SubscriptionExchange::MCXFO,
    //     //     vec!["427035"],
    //     // )
    //     .build()
    //     .unwrap();
    // ws_stream.subscribe(subscription_message).await.unwrap();

    // while let Some(m) = ws_stream.next().await {
    //     // println!("Message is {:?}", m);
    //     match m {
    //         Ok(msg) => {
    //             let current_price = msg.last_traded_price as f64 / 100.0;
    //             println!("Ltp : {:.2?}", (current_price));

    //             if (current_price - last_price).abs() >= price_change_threshold {
    //                 last_price = current_price;
    //                 let token = msg.token.clone(); // Assuming msg has a token field
    //                 let token_clone = msg.token.clone(); // Assuming msg has a token field
    //                 let exchange = msg.exchange.clone();
    //                 println!("\n\nMSG = {:?}", msg);
    //                 tx_main
    //                     .send(Signal::PriceFeed {
    //                         token: msg.token.clone(),
    //                         ltp: last_price,
    //                         is_index: INDICES_TOKENS.contains(&msg.token.as_str()),
    //                     })
    //                     .await;

    //                 // tx_ws_clone.send(Signal::AddNewTradeEngine(TradeEngineConfig {
    //                 //     trade_engine_id: 1,
    //                 //     max_loss,
    //                 //     max_price,
    //                 //     max_trades,
    //                 //     symbol: "SILVERMIC NOV FUT".to_string(),
    //                 //     strategy: Strategy::DynamicChannel,
    //                 //     segment: Segment::FUTURE,
    //                 //     sl: 50.0,
    //                 //     trigger_price,
    //                 //     quantity,
    //                 //     transaction_type,
    //                 //     auth_token: "".to_string(),
    //                 // })).await;
    //             }
    //         }
    //         Err(e) => {
    //             println!("\n\nError with Price feed stream {:?}", e);
    //         }
    //     }
    // }

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
        // if !ws_ready {
        //     if check_server_status(&status_url.as_str(), Duration::from_secs(2)).await {
        //         // Start the WebSocket client and pass the `rx` receiver to receive messages
        //         WebSocketClient::connect(ws_url, rx_ws, tx_main_clone, auth_token).await;
        //     } else {
        //         println!("\n\nServer is offline, retrying in 5 seconds...");
        //         sleep(Duration::from_secs(5)).await;
        //     }
        // }

        std::thread::sleep(std::time::Duration::from_secs(5));
        // println!("Main function is still running...");
    }
}

/*
// Define the type of events
#[derive(Debug, Clone)]
enum TradeEvent {
    PriceFeed {
        price: f64,
        token: String,
        exchange: SubscriptionExchange,
        is_indice: bool,
    },
    // You can define more events as needed
}

// Define the event listener
type EventListener = Box<dyn Fn(&TradeEvent) + Send + Sync>;

/// Event Emitter
struct EventEmitter {
    listeners: HashMap<String, Vec<EventListener>>,
}

impl EventEmitter {
    fn new() -> Self {
        Self {
            listeners: HashMap::new(),
        }
    }

    fn on(&mut self, event_name: &str, listener: Box<dyn Fn(&TradeEvent) + Send + Sync>) {
        self.listeners
            .entry(event_name.to_string())
            .or_insert_with(Vec::new)
            .push(listener);
    }

    fn emit(&self, event_name: &str, event: TradeEvent) {
        if let Some(listeners) = self.listeners.get(event_name) {
            for listener in listeners {
                listener(&event);
            }
        }
    }
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

async fn add_trade_handler(
    // shared_tradehandlers: &mut Vec<TradeEngine>,
    trade_handler_id: u32,
    api_key: &String,
    client_code: &String,
    pin: &String,
    jwt_token: &String,
    refresh_token: &String,
    feed_token: &String,
    max_loss: f64,
    max_price: f64,
    max_trades: u32,
    symbol: String,
    strategy: Strategy,
    segment: Segment,
    quantity: u32,
    sl: f64,
    trigger_price: f64,
    transaction_type: TransactionType,
) -> Result<TradeEngine> {
    println!("Adding handlers...");
    let new_handler = TradeEngine::new_from_session(TradeEngineConfig {
        trade_handler_id: trade_handler_id,
        api_key: api_key.clone(),
        client_code: client_code.clone(),
        pin: pin.clone(),
        jwt_token: jwt_token.clone(),
        refresh_token: refresh_token.clone(),
        feed_token: feed_token.clone(),
        max_loss,
        max_price,
        max_trades,
        symbol,
        strategy,
        segment,
        quantity,
        sl,
        trigger_price,
        transaction_type,
    })
    .await
    .unwrap();
    // let mut h_clone = new_handler.clone();
    // h_clone.profile = h_clone.client.profile().await.unwrap();
    // let mut sc = h_clone.client.lock().await;
    // sc.user = Some(sc.profile().await.unwrap());
    // shared_tradehandlers.push(new_handler.clone());
    // println!("HANDLER = {:?}", new_handler.clone());
    println!("Client code = {:?}", new_handler.client.client_code);
    Ok(new_handler.clone())
}

fn create_message_handler(
    instances: Arc<Mutex<Vec<TradeEngine>>>,
) -> Arc<Mutex<dyn Fn(NewMessage) + Send + Sync>> {
    Arc::new(Mutex::new(move |message: NewMessage| {
        let mut instances = instances.clone();
        tokio::spawn(async move {
            handle_ws_message(&message, &mut instances).await;
        });
    }))
}

async fn handle_ws_message(message: &NewMessage, instances: &mut Arc<Mutex<Vec<TradeEngine>>>) {
    match &message.signal {
        Signal::NewTradeEngine => {
            match &message.data {
                Value::Array(trades) => {
                    let data_received: Vec<TradeEngineConfig> = trades
                        .iter()
                        .map(|trade| serde_json::from_value(trade.clone()).unwrap())
                        .collect();

                    for config in data_received {
                        let mut inst = instances.lock().await;

                        if let Some(existing_handler) = inst
                            .iter()
                            .find(|handler| handler.profile.client_code == config.client_code)
                        {
                            // Check if a handler with matching strategy also exists
                            let strategy_match = inst.iter().find(|handler| {
                                handler.profile.client_code == config.client_code
                                    && handler.strategy == config.strategy
                            });

                            if let Some(handler_with_strategy) = strategy_match {
                                println!("Found handler with matching client_code and strategy: {} - {:?}", handler_with_strategy.profile.client_code, handler_with_strategy.strategy);
                            } else {
                                // If no handler with matching strategy, clone and push the existing one
                                let mut new_handler = existing_handler.clone();
                                new_handler.strategy = config.strategy.clone();
                                inst.push(new_handler);
                                println!("Cloned and pushed new handler with updated strategy: {:?} for client_code: {}", config.strategy, config.client_code);
                            }
                        } else {
                            println!(
                                "No matching handler found for client_code: {}",
                                config.client_code
                            );
                            let new_handler = TradeEngine::new_from_session(config).await.unwrap();
                            inst.push(new_handler);
                        }
                    }
                }
                _ => {
                    println!("Expected an array of trades");
                }
            }
        }
        Signal::Connection => {
            println!("Connection : {:?}", message.data);
        }
        Signal::Auth => {
            println!("{:?}", message.data);
        }
        _ => {
            println!("Other Data received");
        }
    }
}

fn send_message(msg: String, tx: Sender<WebSocketMessage>) {
    // Example of sending messages to the WebSocket from different parts of the program
    tokio::spawn(async move {
        let tx_clone = tx.clone();
        message_sender(tx_clone, msg.into());
        // tokio::time::sleep(Duration::from_secs(5)).await;
        // send_message(tx, "Hello from task 2".into());
    });
}

async fn test_join() {
    // Create a new HTTP client
    let client = Client::new();

    // Define the JSON body
    let body = json!({
        "name": "Apple MacBook Pro 16",
        "data": {
            "year": 2019,
            "price": 1849.99,
            "CPU model": "Intel Core i9",
            "Hard disk size": "1 TB"
        }
    });

    // Send the POST request
    let response = client
        .post("https://api.restful-api.dev/objects")
        .json(&body)
        .send()
        .await;

    // Check if the request was successful
    // if response.status().is_success() {
    //     println!("Successfully posted the data!");
    // } else {
    //     println!("Failed to post the data. Status: {}", response.status());
    // }
    match response {
        Ok(res) => {
            if res.status().is_success() {
                println!("Successfully posted the data!");
            }
        }
        Err(e) => {
            println!("ERROR PROCESSING DATA...");
        }
    }
}

async fn test_handle(client: &SmartConnect) {
    // let client = SmartConnect::new_with_session(api_key, client_code, pin, jwt_token, refresh_token, feed_token).await.unwrap();

    let order_req = PlaceOrderReq::new(
        "MBAPL-EQ",
        "12686",
        aidynamics_trade::types::TransactionType::BUY,
    )
    .quantity(66)
    .product_type(ProductType::IntraDay);
    let resp = match client.place_order(&order_req).await {
        Ok(resp) => {
            println!("{:?}", resp);
        }
        Err(e) => {
            println!("Error occured {:?}", e);
        }
    };
}

// async fn handle_place_order(handler: &mut TradeEngine) {
//     // let mut place_order_instances = instances.lock().await;
//     if handler.trade_status == TradeStatus::Closed {
//         let entry_req = PlaceOrderReq::new(
//             "KOKUYOCMLN-EQ",
//             "16827",
//             aidynamics_trade::types::TransactionType::BUY,
//         )
//         .quantity(10)
//         .product_type(ProductType::IntraDay);
//         handler.entry_req = Some(Arc::new(entry_req));
//         match handler.place_trade().await {
//             Ok(res) => {
//                 if let Some(entry_res) = handler.get_entry_res() {
//                     // FETCHING ORDER STATUS BEGIN

//                     // // // You now have a cloned Arc<PlaceOrderRes> that you can use
//                     // let value = &*entry_res; // Dereference to access the inner PlaceOrderRes
//                     // let unique_order_id = Some(value.unique_order_id.as_ref()).unwrap().unwrap();

//                     // match handler.client.order_status(unique_order_id).await {
//                     //     Ok(resp) => {
//                     //         handler.position = Some(Arc::new(resp));
//                     //         // println!("INSIDE POS : {:?}", handler.position);
//                     //         // if handler.position.is_some() {
//                     //         if let Some(ref position) = handler.position {
//                     //             // let pos = handler.position.clone();
//                     //             // println!("POSITION = {:?}", pos.unwrap());

//                     //             let order_res = &*position.clone();
//                     //             handler.trade_entry_price = order_res.order.price;

//                     //             // let client = Arc::new(Client::new());
//                     //             // let client_clone = Arc::clone(&client);

//                     //             // let url = end_points::order_url();

//                     //             // let client_code = handler.client.client_code.clone();
//                     //             // let payload = Payload {
//                     //             //     request_type: "new_order".to_string(),
//                     //             //     data: serde_json::json!({"client_code":client_code,"unique_order_id":unique_order_id})
//                     //             // };
//                     //             // let response = client_clone.post(url).json(&payload).send().await;

//                     //             // match response {
//                     //             //     Ok(res) => {
//                     //             //         // Check if the response status is successful
//                     //             //         if res.status().is_success() {
//                     //             //             let trade = res.json::<Trade>().await.unwrap();
//                     //             //             println!("Trade: {:?}", trade);
//                     //             //             // Ok(trade)
//                     //             //         } else {
//                     //             //             // Handle non-successful status codes
//                     //             //             println!("Failed to send order status. HTTP Status: {}", res.status());
//                     //             //             // Err("Failed to send order status. HTTP Status:"
//                     //             //             //     .into())
//                     //             //         }
//                     //             //     }
//                     //             //     Err(e) => {
//                     //             //         // Handle the error that occurred during the request
//                     //             //         eprintln!("Request error: {:?}", e);
//                     //             //         // Err(Box::new(e))
//                     //             //     }
//                     //             // }

//                     //             // handler.update_order("new_order", unique_order_id.clone()).await.unwrap();
//                     //         }
//                     //     }
//                     //     // Err(e) => {
//                     //     //     println!(
//                     //     //         "Error occured in getting order_status from angelone.... {:?}",
//                     //     //         e
//                     //     //     );
//                     //     //     return Err(format!(
//                     //     //         "Error occured in getting order_status from angelone... {:?}",
//                     //     //         e
//                     //     //     ));
//                     //     // }
//                     //     Err(e) => {
//                     //         println!("Error placing order: {:?}", e);
//                     //     }
//                     // }

//                     // FETCHING ORDER STATUS END

//                     // let client = Arc::new(Client::new());
//                     // let client_clone = Arc::clone(&client);

//                     // let url = end_points::order_url();
//                     // println!("Url = {}", url);

//                     // let client_code = handler.client.client_code.clone();
//                     // let payload = Payload {
//                     //     request_type: "new_order".to_string(),
//                     //     data: serde_json::json!({"client_code":client_code,"unique_order_id":unique_order_id,"sl":handler.trade_sl_points,"strategy":handler.strategy}),
//                     // };
//                     // let response = client_clone.post(url).json(&payload).send().await;
//                     // match response {
//                     //     Ok(res) => {
//                     //         // Check if the response status is successful
//                     //         if res.status().is_success() {
//                     //             let trade = res.json::<Trade>().await.unwrap();
//                     //             println!("Trade: {:?}", trade);
//                     //             // Ok(trade)
//                     //         } else {
//                     //             // Handle non-successful status codes
//                     //             println!(
//                     //                 "Failed to send order status. HTTP Status: {}",
//                     //                 res.status()
//                     //             );
//                     //             // Err("Failed to send order status. HTTP Status:"
//                     //             //     .into())
//                     //         }
//                     //     }
//                     //     Err(e) => {
//                     //         // Handle the error that occurred during the request
//                     //         eprintln!("Request error: {:?}", e);
//                     //         // Err(Box::new(e))
//                     //     }
//                     // }
//                 } else {
//                     println!("No entry_res found");
//                     // return Err("No entry_res found".to_string());
//                 }

//                 // println!("ENTRY RESPONSE = {:?}", handler.entry_res);
//                 // if let Some(entry_res) = handler.get_entry_res() {
//                 //     println!("Entry done");
//                 //     // // You now have a cloned Arc<PlaceOrderRes> that you can use
//                 //     // let value = &*entry_res; // Dereference to access the inner PlaceOrderRes
//                 //     // let unique_order_id = Some(value.unique_order_id.as_ref()).unwrap().unwrap();

//                 //     // match handler.client.order_status(unique_order_id).await {
//                 //     //     Ok(resp) => {
//                 //     //         handler.position = Some(Arc::new(resp));
//                 //     //         println!("INSIDE POS : {:?}", handler.position);
//                 //     //         // // if handler.position.is_some() {
//                 //     //         // if let Some(ref position) = handler.position {
//                 //     //         //     let pos = handler.position.clone();
//                 //     //         //     println!("POSITION = {:?}", pos.unwrap());

//                 //     //         //     // let order_res = &*position.clone();
//                 //     //         //     // handler.trade_entry_price = order_res.order.price;

//                 //     //         //     // let client = Arc::new(Client::new());
//                 //     //         //     // let client_clone = Arc::clone(&client);

//                 //     //         //     // let url = end_points::order_url();

//                 //     //         //     // let client_code = handler.client.client_code.clone();
//                 //     //         //     // let payload = Payload {
//                 //     //         //     //     request_type: "new_order".to_string(),
//                 //     //         //     //     data: serde_json::json!({"client_code":client_code,"unique_order_id":unique_order_id})
//                 //     //         //     // };
//                 //     //         //     // let response = client_clone.post(url).json(&payload).send().await;

//                 //     //         //     // match response {
//                 //     //         //     //     Ok(res) => {
//                 //     //         //     //         // Check if the response status is successful
//                 //     //         //     //         if res.status().is_success() {
//                 //     //         //     //             let trade = res.json::<Trade>().await.unwrap();
//                 //     //         //     //             println!("Trade: {:?}", trade);
//                 //     //         //     //             // Ok(trade)
//                 //     //         //     //         } else {
//                 //     //         //     //             // Handle non-successful status codes
//                 //     //         //     //             println!("Failed to send order status. HTTP Status: {}", res.status());
//                 //     //         //     //             // Err("Failed to send order status. HTTP Status:"
//                 //     //         //     //             //     .into())
//                 //     //         //     //         }
//                 //     //         //     //     }
//                 //     //         //     //     Err(e) => {
//                 //     //         //     //         // Handle the error that occurred during the request
//                 //     //         //     //         eprintln!("Request error: {:?}", e);
//                 //     //         //     //         // Err(Box::new(e))
//                 //     //         //     //     }
//                 //     //         //     // }

//                 //     //         //     // handler.update_order("new_order", unique_order_id.clone()).await.unwrap();
//                 //     //         // }
//                 //     //     }
//                 //     //     Err(e) => {
//                 //     //         println!(
//                 //     //             "Error occured in getting order_status from angelone.... {:?}",
//                 //     //             e
//                 //     //         );
//                 //     //     }
//                 //     // }

//                 //     // let client = Arc::new(Client::new());
//                 //     // let client_clone = Arc::clone(&client);

//                 //     // let url = end_points::order_url();
//                 //     // println!("Url = {}", url);

//                 //     // let client_code = handler.client.client_code.clone();
//                 //     // let payload = Payload {
//                 //     //     request_type: "new_order".to_string(),
//                 //     //     data: serde_json::json!({"client_code":client_code,"unique_order_id":unique_order_id,"sl":handler.trade_sl_points,"strategy":handler.strategy}),
//                 //     // };
//                 //     // let response = client_clone.post(url).json(&payload).send().await;
//                 //     // match response {
//                 //     //     Ok(res) => {
//                 //     //         // Check if the response status is successful
//                 //     //         if res.status().is_success() {
//                 //     //             let trade = res.json::<Trade>().await.unwrap();
//                 //     //             println!("Trade: {:?}", trade);
//                 //     //             // Ok(trade)
//                 //     //         } else {
//                 //     //             // Handle non-successful status codes
//                 //     //             println!(
//                 //     //                 "Failed to send order status. HTTP Status: {}",
//                 //     //                 res.status()
//                 //     //             );
//                 //     //             // Err("Failed to send order status. HTTP Status:"
//                 //     //             //     .into())
//                 //     //         }
//                 //     //     }
//                 //     //     Err(e) => {
//                 //     //         // Handle the error that occurred during the request
//                 //     //         eprintln!("Request error: {:?}", e);
//                 //     //         // Err(Box::new(e))
//                 //     //     }
//                 //     // }
//                 // } else {
//                 //     println!("No entry_res found");
//                 // }
//             }
//             Err(e) => {
//                 println!("ERROR PLACING ORDER {:?}", e);
//             }
//         };
//     }
// }

async fn handle_place_order(handler: &mut TradeEngine) {
    // let mut place_order_instances = instances.lock().await;
    if handler.trade_status == TradeStatus::Closed {
        handler.wait = true;
        let entry_req = PlaceOrderReq::new(
            // "BANKNIFTY28AUG2451100CE",
            // "48006",
            "KOKUYOCMLN-EQ",
            "16827",
            aidynamics_trade::types::TransactionType::BUY,
        )
        .exchange(ExchangeType::NSE)
        .quantity(15)
        .product_type(ProductType::IntraDay);
        handler.entry_req = Some(Arc::new(entry_req));
        match handler.place_trade().await {
            Ok(res) => {
                if let Some(entry_res) = handler.get_entry_res() {
                    // FETCHING ORDER STATUS BEGIN

                    // // // You now have a cloned Arc<PlaceOrderRes> that you can use
                    // let value = &*entry_res; // Dereference to access the inner PlaceOrderRes
                    // let unique_order_id = Some(value.unique_order_id.as_ref()).unwrap().unwrap();

                    // match handler.client.order_status(unique_order_id).await {
                    //     Ok(resp) => {
                    //         handler.position = Some(Arc::new(resp));
                    //         // println!("INSIDE POS : {:?}", handler.position);
                    //         // if handler.position.is_some() {
                    //         if let Some(ref position) = handler.position {
                    //             // let pos = handler.position.clone();
                    //             // println!("POSITION = {:?}", pos.unwrap());

                    //             let order_res = &*position.clone();
                    //             handler.trade_entry_price = order_res.order.price;

                    //             // let client = Arc::new(Client::new());
                    //             // let client_clone = Arc::clone(&client);

                    //             // let url = end_points::order_url();

                    //             // let client_code = handler.client.client_code.clone();
                    //             // let payload = Payload {
                    //             //     request_type: "new_order".to_string(),
                    //             //     data: serde_json::json!({"client_code":client_code,"unique_order_id":unique_order_id})
                    //             // };
                    //             // let response = client_clone.post(url).json(&payload).send().await;

                    //             // match response {
                    //             //     Ok(res) => {
                    //             //         // Check if the response status is successful
                    //             //         if res.status().is_success() {
                    //             //             let trade = res.json::<Trade>().await.unwrap();
                    //             //             println!("Trade: {:?}", trade);
                    //             //             // Ok(trade)
                    //             //         } else {
                    //             //             // Handle non-successful status codes
                    //             //             println!("Failed to send order status. HTTP Status: {}", res.status());
                    //             //             // Err("Failed to send order status. HTTP Status:"
                    //             //             //     .into())
                    //             //         }
                    //             //     }
                    //             //     Err(e) => {
                    //             //         // Handle the error that occurred during the request
                    //             //         eprintln!("Request error: {:?}", e);
                    //             //         // Err(Box::new(e))
                    //             //     }
                    //             // }

                    //             // handler.update_order("new_order", unique_order_id.clone()).await.unwrap();
                    //         }
                    //     }
                    //     // Err(e) => {
                    //     //     println!(
                    //     //         "Error occured in getting order_status from angelone.... {:?}",
                    //     //         e
                    //     //     );
                    //     //     return Err(format!(
                    //     //         "Error occured in getting order_status from angelone... {:?}",
                    //     //         e
                    //     //     ));
                    //     // }
                    //     Err(e) => {
                    //         println!("Error placing order: {:?}", e);
                    //     }
                    // }

                    // FETCHING ORDER STATUS END

                    // let client = Arc::new(Client::new());
                    // let client_clone = Arc::clone(&client);

                    // let url = end_points::order_url();
                    // println!("Url = {}", url);

                    // let client_code = handler.client.client_code.clone();
                    // let payload = Payload {
                    //     request_type: "new_order".to_string(),
                    //     data: serde_json::json!({"client_code":client_code,"unique_order_id":unique_order_id,"sl":handler.trade_sl_points,"strategy":handler.strategy}),
                    // };
                    // let response = client_clone.post(url).json(&payload).send().await;
                    // match response {
                    //     Ok(res) => {
                    //         // Check if the response status is successful
                    //         if res.status().is_success() {
                    //             let trade = res.json::<Trade>().await.unwrap();
                    //             println!("Trade: {:?}", trade);
                    //             // Ok(trade)
                    //         } else {
                    //             // Handle non-successful status codes
                    //             println!(
                    //                 "Failed to send order status. HTTP Status: {}",
                    //                 res.status()
                    //             );
                    //             // Err("Failed to send order status. HTTP Status:"
                    //             //     .into())
                    //         }
                    //     }
                    //     Err(e) => {
                    //         // Handle the error that occurred during the request
                    //         eprintln!("Request error: {:?}", e);
                    //         // Err(Box::new(e))
                    //     }
                    // }
                } else {
                    println!("No entry_res found");
                    // return Err("No entry_res found".to_string());
                }

                // println!("ENTRY RESPONSE = {:?}", handler.entry_res);
                // if let Some(entry_res) = handler.get_entry_res() {
                //     println!("Entry done");
                //     // // You now have a cloned Arc<PlaceOrderRes> that you can use
                //     // let value = &*entry_res; // Dereference to access the inner PlaceOrderRes
                //     // let unique_order_id = Some(value.unique_order_id.as_ref()).unwrap().unwrap();

                //     // match handler.client.order_status(unique_order_id).await {
                //     //     Ok(resp) => {
                //     //         handler.position = Some(Arc::new(resp));
                //     //         println!("INSIDE POS : {:?}", handler.position);
                //     //         // // if handler.position.is_some() {
                //     //         // if let Some(ref position) = handler.position {
                //     //         //     let pos = handler.position.clone();
                //     //         //     println!("POSITION = {:?}", pos.unwrap());

                //     //         //     // let order_res = &*position.clone();
                //     //         //     // handler.trade_entry_price = order_res.order.price;

                //     //         //     // let client = Arc::new(Client::new());
                //     //         //     // let client_clone = Arc::clone(&client);

                //     //         //     // let url = end_points::order_url();

                //     //         //     // let client_code = handler.client.client_code.clone();
                //     //         //     // let payload = Payload {
                //     //         //     //     request_type: "new_order".to_string(),
                //     //         //     //     data: serde_json::json!({"client_code":client_code,"unique_order_id":unique_order_id})
                //     //         //     // };
                //     //         //     // let response = client_clone.post(url).json(&payload).send().await;

                //     //         //     // match response {
                //     //         //     //     Ok(res) => {
                //     //         //     //         // Check if the response status is successful
                //     //         //     //         if res.status().is_success() {
                //     //         //     //             let trade = res.json::<Trade>().await.unwrap();
                //     //         //     //             println!("Trade: {:?}", trade);
                //     //         //     //             // Ok(trade)
                //     //         //     //         } else {
                //     //         //     //             // Handle non-successful status codes
                //     //         //     //             println!("Failed to send order status. HTTP Status: {}", res.status());
                //     //         //     //             // Err("Failed to send order status. HTTP Status:"
                //     //         //     //             //     .into())
                //     //         //     //         }
                //     //         //     //     }
                //     //         //     //     Err(e) => {
                //     //         //     //         // Handle the error that occurred during the request
                //     //         //     //         eprintln!("Request error: {:?}", e);
                //     //         //     //         // Err(Box::new(e))
                //     //         //     //     }
                //     //         //     // }

                //     //         //     // handler.update_order("new_order", unique_order_id.clone()).await.unwrap();
                //     //         // }
                //     //     }
                //     //     Err(e) => {
                //     //         println!(
                //     //             "Error occured in getting order_status from angelone.... {:?}",
                //     //             e
                //     //         );
                //     //     }
                //     // }

                //     // let client = Arc::new(Client::new());
                //     // let client_clone = Arc::clone(&client);

                //     // let url = end_points::order_url();
                //     // println!("Url = {}", url);

                //     // let client_code = handler.client.client_code.clone();
                //     // let payload = Payload {
                //     //     request_type: "new_order".to_string(),
                //     //     data: serde_json::json!({"client_code":client_code,"unique_order_id":unique_order_id,"sl":handler.trade_sl_points,"strategy":handler.strategy}),
                //     // };
                //     // let response = client_clone.post(url).json(&payload).send().await;
                //     // match response {
                //     //     Ok(res) => {
                //     //         // Check if the response status is successful
                //     //         if res.status().is_success() {
                //     //             let trade = res.json::<Trade>().await.unwrap();
                //     //             println!("Trade: {:?}", trade);
                //     //             // Ok(trade)
                //     //         } else {
                //     //             // Handle non-successful status codes
                //     //             println!(
                //     //                 "Failed to send order status. HTTP Status: {}",
                //     //                 res.status()
                //     //             );
                //     //             // Err("Failed to send order status. HTTP Status:"
                //     //             //     .into())
                //     //         }
                //     //     }
                //     //     Err(e) => {
                //     //         // Handle the error that occurred during the request
                //     //         eprintln!("Request error: {:?}", e);
                //     //         // Err(Box::new(e))
                //     //     }
                //     // }
                // } else {
                //     println!("No entry_res found");
                // }
            }
            Err(e) => {
                println!("ERROR PLACING ORDER {:?}", e);
            }
        };
    }
}

async fn place_order(
    handler: &mut TradeEngine,
) -> std::result::Result<TradeEngine, Box<dyn std::error::Error>> {
    if handler.trade_status == TradeStatus::Closed {
        handler.wait = true;
        let entry_req = PlaceOrderReq::new(
            "KOKUYOCMLN-EQ",
            "16827",
            aidynamics_trade::types::TransactionType::BUY,
        )
        .exchange(ExchangeType::NSE)
        .quantity(15)
        .product_type(ProductType::IntraDay);

        handler.entry_req = Some(Arc::new(entry_req));
        let mut handler_clone = handler.clone();

        // Spawn the task using `tokio::spawn`
        let task = tokio::task::spawn(async move {
            match handler_clone.place_trade().await {
                Ok(res) => {
                    if let Some(entry_res) = handler_clone.get_entry_res() {
                        // Process your order status here as needed
                        // If order status fetching is needed, do it here
                        // You can update the handler's variants as per your logic
                    } else {
                        println!("No entry_res found");
                    }
                }
                Err(e) => {
                    println!("ERROR PLACING ORDER {:?}", e);
                }
            }
            handler_clone // Return the modified handler
        });

        // Await the task to get the result
        match task.await {
            Ok(mut handler) => Ok(handler),
            Err(e) => Err(Box::new(e) as Box<dyn std::error::Error>),
        }
    } else {
        // If the trade status is not closed, return the handler as is
        Ok(handler.to_owned())
    }
}

// async fn handle_price_feed(shared_instance:&mut Arc<Mutex<Vec<TradeEngine>>>,msg:Message){
//     let result: f64 = msg.last_traded_price as f64 / 100.0;
//     println!("Ltp : {:.2?}",(result));

// }

async fn login_web_server(
    auth_token: &mut String,
    angelone_tokens: &mut Option<Value>,
    rust_password: &String,
) -> Result<()> {
    // Create an HTTP client
    let client = Client::new();

    // Define the URL to send the POST request to
    let url = "http://localhost:8000/serverlogin"; // Example URL, replace with your actual endpoint

    // Define the payload for the POST request
    let payload = json!({
        "client_id": "RUST",
        "password":rust_password
    });

    // Send the POST request with JSON payload
    let response = client.post(url).json(&payload).send().await?;
    // Check if the request was successful
    if response.status().is_success() {
        let response_json: Value = response.json().await?;
        println!("\n\nResponse: {}\n\n", response_json);
        if let Some(status) = response_json.get("status") {
            if status == "success" {
                if let Some(token) = response_json.get("server_token") {
                    if let Some(token_str) = token.as_str() {
                        println!("\n\nACTUAL SERVER TOKEN {:?}\n\n", token_str);
                        println!("\n\nCONVERTING TO STRING {:?}\n\n", token_str.to_string());
                        *auth_token = token_str.to_string();
                    }
                } else {
                    println!("Error fetching tokens");
                }
                if let Some(ao_token) = response_json.get("angelone_tokens") {
                    *angelone_tokens = Some(ao_token.clone());
                }
            } else {
                println!("Error {:?}", Some(response_json.get("message")));
            }
        }
    } else {
        eprintln!("Failed to send POST request. Status: {}", response.status());
    }
    Ok(())
}


async fn sync_tradehandler(rx: &mut Receiver<TradeEngine>,trade_handler_ref:&mut Arc<Mutex<Vec<TradeEngine>>>) {
    while let Some(handler) = rx.recv().await {
        let trade_handler_updater = Arc::clone(trade_handler_ref);
        let trade_handlers_clone = trade_handler_updater.clone();
        let handlers = trade_handlers_clone.lock().await;
        for handler in handlers{

        }
    }
}

#[derive(Clone)]
struct PriceFeedEvent {
    price: f64,
    token: String,
    exchange: SubscriptionExchange,
}

#[tokio::main]
async fn main() {
    let (tx_tradehandler,mut rx_tradehandler) = tokio::sync::mpsc::channel::<TradeEngine>(100);



    let api_key = dotenv::var("API_KEY").unwrap();
    let client_code = dotenv::var("CLIENT_CODE").unwrap();
    let pin = dotenv::var("PIN").unwrap();
    let otp_token = dotenv::var("OTP_TOKEN").unwrap();
    let firebase_url = dotenv::var("FIREBASE_URL").unwrap();
    let ws_url = dotenv::var("WS_URL").unwrap();
    let status_url = dotenv::var("STATUS_URL").unwrap();
    let rust_password = dotenv::var("NEW_RUST_PASSWORD").unwrap();
    // BANKNIFTY = 26009,NIFTY = 26000, FINNIFTY = 26037
    // const indices_tokens: [&'static str; 3] = ["26009", "26000", "26037"];
    const INDICES_TOKENS: [&'static str; 3] = ["26009", "26000", "26037"];

    let mut shared_tradehandlers = Arc::new(Mutex::new(Vec::<TradeEngine>::new()));
    let mut auth_token = "".to_string();
    // let mut web_server_token = "";
    let mut angelone_tokens: Option<Value> = None;
    // let mut jwt_token: String = dotenv::var("JWT_TOKEN").unwrap();
    // let mut feed_token: String = dotenv::var("FEED_TOKEN").unwrap();
    // let mut refresh_token = dotenv::var("REFRESH_TOKEN").unwrap();
    let mut jwt_token: String = "".to_string();
    let mut feed_token: String = "".to_string();
    let mut refresh_token = "".to_string();
    let s_instances = Arc::clone(&shared_tradehandlers);

    let api_key_clone = api_key.clone();
    let client_code_clone = client_code.clone();
    let pin_clone = pin.clone();
    // let mut jwt_token_clone = jwt_token.clone();
    // let mut  feed_token_clone = feed_token.clone();
    // let mut refresh_token_clone = refresh_token.clone();
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

    // integrate_firebase(&firebase_url).await;

    login_web_server(&mut auth_token, &mut angelone_tokens, &rust_password).await;

    if let Some(ao_tokens) = angelone_tokens {
        if let Some(jt) = ao_tokens.get("jwt_token") {
            if let Some(jt_str) = jt.as_str(){
                jwt_token = jt_str.to_string();
            }
            // println!("\n\n JWT TOKEN IS {:?} \n\n", jwt_token);
        }
        if let Some(rt) = ao_tokens.get("refresh_token") {
            if let Some(rt_str) = rt.as_str(){
                refresh_token = rt_str.to_string();
            }
            // println!("\n\n REFRESH TOKEN IS {:?} \n\n", refresh_token);
        }
        if let Some(ft) = ao_tokens.get("feed_token") {

            if let Some(ft_str) = ft.as_str(){
                feed_token = ft_str.to_string();
            }
            // println!("\n\n FEED TOKEN IS {:?} \n\n", feed_token);
        }
    }

    let jwt_token_clone = jwt_token.clone();
    let refresh_token_clone = refresh_token.clone();
    let feed_token_clone = feed_token.clone();

    // Spawn the task, passing a mutable reference
    let mut shared_thnd = Arc::clone(&shared_tradehandlers);
    tokio::spawn(async move {
        sync_tradehandler(&mut rx_sync,&mut shared_thnd).await;
    });

    tokio::spawn(async move {
        {
            println!("Program started....");
            let mut instances = s_instances.lock().await;
            for i in 0..3 {
                let t_handler = add_trade_handler(
                    i,
                    &api_key_clone,
                    &client_code_clone,
                    &pin_clone,
                    &jwt_token_clone,
                    &refresh_token_clone,
                    &feed_token_clone,
                    max_loss,
                    max_price,
                    max_trades,
                    "BANKNIFTY".to_string(),
                    Strategy::Momentum,
                    Segment::OPTION,
                    quantity,
                    trade_sl_points,
                    trigger_price,
                    transaction_type,
                )
                .await
                .unwrap();
                instances.push(t_handler.clone());
            }
            // println!("All loops done...");
        }
    })
    .await
    .unwrap();

    println!("Started.......");
    let start = Instant::now();
    // Split the orders into batches
    let mut tasks = vec![];

    // {
    //     // let place_order_instance = Arc::clone(&shared_tradehandlers);
    //     // let mut handlers = place_order_instance.lock().await;
    //     let mut handlers = shared_tradehandlers.lock().await;

    //     let mut batches = handlers.chunks_mut(50);
    //     println!("Created batches....");
    //     // Split the instances into chunks of 50
    //     for batch in batches {
    //         // Convert the chunk to a Vec for ownership, ensuring that each chunk is independent
    //         let mut batch = batch.to_vec();
    //         let mut place_order_instance_clone = Arc::clone(&shared_tradehandlers);
    //         println!("Created : place_order_instance_clone");
    //         for mut handler in batch {
    //             // Spawn a new task to handle each chunk concurrently
    //             tasks.push(tokio::task::spawn(async move {
    //                 // for mut handler in batch {
    //                     let place_order_instance_clone = Arc::clone(&place_order_instance_clone);
    //                     println!("Created : place_order_instance_clone 2");
    //                     handle_place_order(&mut handler).await;
    //                     let mut handlers = place_order_instance_clone.lock().await;
    //                     if let Some(item) = handlers
    //                         .iter_mut()
    //                         .find(|x| x.trade_handler_id == handler.trade_handler_id)
    //                     {
    //                         item.trade_entry_price = handler.trade_entry_price;
    //                         item.entry_res = handler.entry_res.clone();
    //                         item.position = handler.position.clone();
    //                     }
    //                 // }
    //             }));
    //         }
    //     }
    // }

    // {
    // Acquire the lock just to split into batches and then release it
    let handlers = {
        let handlers = shared_tradehandlers.lock().await;
        handlers.clone() // Clone the entire vector so we can release the lock
    };

    // let handlers = shared_tradehandlers.lock().await;
    let batches = handlers.chunks(50);

    for batch in batches {
        let batch = batch.to_vec(); // Clone the batch for ownership
        let place_order_instance_clone = Arc::clone(&shared_tradehandlers);
        tasks.push(tokio::task::spawn(async move {
            for mut handler in batch {
                // Spawn a task to process each handler individually
                let place_order_instance_clone = Arc::clone(&place_order_instance_clone);
                tokio::task::spawn(async move {
                    let mut processed_handlers = Vec::new();
                    // handle_place_order(&mut handler).await;
                    place_order(&mut handler).await;
                    processed_handlers.push(handler);

                    let mut shared_handlers = place_order_instance_clone.lock().await;
                    for processed_handler in processed_handlers {
                        if let Some(original_handler) = shared_handlers
                            .iter_mut()
                            .find(|h| h.trade_handler_id == processed_handler.trade_handler_id)
                        {
                            original_handler.entry_res = processed_handler.entry_res.clone();
                        }
                    }

                    // if let Some(item) = handlers.iter_mut().find(|x| x.trade_handler_id == handler.trade_handler_id) {
                    //     println!("Item entry price = {:?}, handler entry price = {:?}",item.trade_entry_price,handler.trade_entry_price);
                    //     item.trade_entry_price = handler.trade_entry_price;
                    //     item.entry_res = handler.entry_res.clone();
                    //     item.position = handler.position.clone();
                    // }

                    // // Spawn another task to update the shared state
                    // let place_order_instance_clone = Arc::clone(&place_order_instance_clone);
                    // tokio::task::spawn(async move {
                    //     let mut handlers = place_order_instance_clone.lock().await;

                    //     for handler in handlers.iter(){
                    //         println!("Entry price = : {:?}, position : {:?}",handler.trade_entry_price,handler.position);
                    //     }

                    //     // if let Some(item) = handlers.iter_mut().find(|x| x.trade_handler_id == handler.trade_handler_id) {
                    //     //     println!("Item entry price = {:?}, handler entry price = {:?}",item.trade_entry_price,handler.trade_entry_price);
                    //     //     item.trade_entry_price = handler.trade_entry_price;
                    //     //     item.entry_res = handler.entry_res.clone();
                    //     //     item.position = handler.position.clone();
                    //     // }
                    // }).await.unwrap();

                    // Ensure inner task is awaited
                })
                .await
                .unwrap(); // Ensure outer task is awaited
            }
        }));
    }
    // } // Lock is released here

    let _results = join_all(tasks).await;
    // Calculate the elapsed time
    let duration = start.elapsed();
    println!("TIME TAKEN = {:?}", duration);

    let mut _tasks = vec![];
    {
        // Spawn another task to update the shared state
        let place_order_instance_clone = Arc::clone(&shared_tradehandlers);
        _tasks.push(tokio::task::spawn(async move {
            {
                let mut handlers = place_order_instance_clone.lock().await;
                for handler in handlers.iter_mut() {
                    if let Some(ref entry_res) = handler.entry_res {
                        let order_res = &*entry_res.clone();
                        if let Some(ref entry_r) = order_res.unique_order_id {
                            let unique_order_id = entry_r.to_string();
                            match handler.client.order_status(unique_order_id.clone()).await {
                                Ok(ord_status) => {
                                    let order_status = Arc::new(ord_status.clone());
                                    handler.position = Some(order_status.clone());
                                    handler.trade_entry_price = order_status.clone().order.price;

                                    // if order_status.order.order_status == "executed" {
                                    let trans_type =
                                        if handler.transaction_type == TransactionType::BUY {
                                            TransactionType::SELL
                                        } else {
                                            TransactionType::BUY
                                        };
                                    let exit_req = PlaceOrderReq::new(
                                        handler.trading_symbol.clone(),
                                        order_status.clone().order.symbol_token.clone(),
                                        trans_type,
                                    )
                                    .exchange(ExchangeType::NSE)
                                    .quantity(order_status.clone().order.quantity.clone())
                                    .product_type(ProductType::IntraDay);
                                    handler.exit_req = Some(Arc::new(exit_req));
                                    // }
                                }
                                Err(e) => {
                                    // return Err(Box::new(e) as Error);
                                    // return Err(e.to_string())
                                    println!("Print error placing trade {:?}", e);
                                    // return Err("No order request provided".to_string());
                                }
                            }
                        }
                    }
                }
            }
        }));
        // .await;
    }
    let _results = join_all(_tasks).await;
    println!("Sleeping....");
    sleep(Duration::from_secs(5)).await;

    {
        // Re-acquire the lock to access the updated shared state
        let handlers = shared_tradehandlers.lock().await;
        for hnd in handlers.iter() {
            println!("Entry response = {:?}", &hnd.entry_res);
            println!("ENTRY PRICE = {:?}", &hnd.trade_entry_price);

            println!("Position = {:?}", &hnd.position);
            println!("EXIT REQ = {:?}", &hnd.exit_req);
        }
    }

    // {
    //     // Re-acquire the lock to access the updated shared state
    //     let handlers = shared_tradehandlers.lock().await;
    //     for hnd in handlers.iter() {
    //         println!("Entry response = {:?}", hnd.entry_res);
    //         println!("Position = {:?}", hnd.position);
    //     }
    // }

    // IMPLEMENTING CLIENTS VEC END

    // Create a channel for sending WebSocket messages
    let (tx, mut rx) = mpsc::channel::<WebSocketMessage>(32);
    let clone_shared_ins = Arc::clone(&shared_tradehandlers);
    let message_handler = create_message_handler(clone_shared_ins);

    // MESSAGE HANDLER COMMENTED TEMPORARY

    tokio::spawn(async move {
        // let handler = message_handler.clone();
        loop {
            if check_server_status(&status_url.as_str(), Duration::from_secs(2)).await {
                println!("Server is online, attempting to connect...");
                let (tx, rx) = mpsc::channel::<WebSocketMessage>(32);
                let handler = message_handler.clone();
                let wtoken = auth_token.clone();

                match connect_to_websocket(&ws_url.as_str(), rx, handler, wtoken).await {
                    Ok(_) => {
                        // let wtoken = web_server_token.clone();
                        println!("Connection closed!");
                    }
                    Err(e) => println!("WebSocket connection failed: {:?}", e),
                }
            } else {
                println!("Server is offline, retrying in 5 seconds...");
                sleep(Duration::from_secs(5)).await;
            }
        }
    });

    // LIVE PRICE FEED WEBSOCKET CONNECTION BEGIN

    // Acquire the lock just to split into batches and then release it
    let mut handlers = {
        let handlers = shared_tradehandlers.lock().await;
        // handlers.clone() // Clone the entire vector so we can release the lock
        let hndl: Vec<TradeEngine> = handlers
            .clone()
            .into_iter()
            .filter(|h| !h.stopped)
            .collect();
        hndl
    };

    let mut event_emitter = EventEmitter::new();

    let mut handlers_c = handlers.clone();
    // Register a listener for the PriceFeed event
    event_emitter.on(
        "PriceFeed",
        Box::new(move |event| {
            if let TradeEvent::PriceFeed {
                price,
                token,
                exchange,
                is_indice,
            } = event
            {
                let price_rx = price.clone();
                let token_rx = token.clone();
                let exchange_rx = exchange.clone();
                let indice_rx = is_indice.clone();
                // println!("Received price feed: {:?} with token: {:?}", price, token);
                let event_clone = event.clone();
                let mut trade_handlers_clone = handlers_c.clone();
                // println!("Event triggered = {:?}", event_clone);
                tokio::spawn(async move {
                    // let handlers = trade_handlers_clone.lock().await;
                    println!(
                        "Price : {:?}, Token : {:?}, Exchange : {:?}",
                        price_rx, token_rx, exchange_rx
                    );

                    for handler in trade_handlers_clone.iter_mut() {
                        if indice_rx {
                            println!("FOUND INDICE : {:?}", price_rx);
                            if handler.trigger_price != 0.0 && handler.wait && handler.trade_id == 0
                            {
                                if handler.transaction_type == TransactionType::BUY
                                    && price_rx > handler.trigger_price
                                {
                                    // handle_place_order(handler).await;
                                    place_order(handler).await.unwrap();
                                } else if handler.transaction_type == TransactionType::SELL
                                    && price_rx < handler.trigger_price
                                    && handler.wait
                                    && handler.trade_id == 0
                                {
                                    // handle_place_order(handler).await;
                                    place_order(handler).await.unwrap();
                                }
                            }
                        }
                        {
                            if handler.transaction_type == TransactionType::BUY
                                && (handler.sl != 0.0
                                    && price_rx <= (handler.trade_entry_price - handler.sl))
                            {
                                handler.squareoff_trade().await.unwrap();
                            } else if handler.transaction_type == TransactionType::SELL
                                && (handler.sl != 0.0
                                    && price_rx >= (handler.sl - handler.trade_entry_price))
                            {
                                handler.squareoff_trade().await.unwrap();
                            }
                        }
                    }
                });
            }
        }),
    );

    let indice_token = vec!["26009".to_string()];
    let ao_ws = AngelOneWs::new(client_code, feed_token);
    let mut last_price: f64 = 0.0;
    let price_change_threshold: f64 = 10.0; // Only emit if the price changes by 10 or more

    let mut ws_stream = ao_ws.stream::<Message>().await.unwrap();

    let subscription_message = SubscriptionBuilder::new("abcde12345")
        // .mode(SubscriptionMode::Quote)
        .mode(SubscriptionMode::Ltp)
        .subscribe(SubscriptionExchange::NSECM, vec!["26009"])
        // .subscribe(
        //     SubscriptionExchange::NSEFO,
        //     vec!["48436", "48444"],
        // )
        //  429116" SILVERMIC24NOVFU
        // .subscribe(SubscriptionExchange::MCXFO, vec!["429116"])
        // .subscribe(
        //     SubscriptionExchange::MCXFO,
        //     vec!["427035"],
        // )
        .build()
        .unwrap();
    ws_stream.subscribe(subscription_message).await.unwrap();
    // let mut lock_counter = 0;
    // let max_lock_count = 25;

    while let Some(m) = ws_stream.next().await {
        // println!("Message is {:?}", m);
        match m {
            Ok(msg) => {
                // lock_counter += 1;
                // if lock_counter == max_lock_count {
                //     lock_counter = 0;

                //     for handler in handlers.iter_mut() {
                //         tokio::spawn(async move {
                //             println!("SLEEPING FOR 5 SECONDS...");
                //             sleep(Duration::from_secs(5)).await;
                //             println!("SLEEP RELEASED...");
                //         });
                //     }
                // }

                let current_price = msg.last_traded_price as f64 / 100.0;
                println!("Ltp : {:.2?}", (current_price));

                if (current_price - last_price).abs() >= price_change_threshold {
                    last_price = current_price;
                    let token = msg.token.clone(); // Assuming msg has a token field
                    let token_clone = msg.token.clone(); // Assuming msg has a token field
                    let exchange = msg.exchange.clone();
                    event_emitter.emit(
                        "PriceFeed",
                        TradeEvent::PriceFeed {
                            price: current_price,
                            token,
                            exchange,
                            is_indice: indice_token.contains(&token_clone),
                        },
                    );
                }

                // handle_price_feed(lock_counter,&mut shared_tradehandlers,msg,max_lock_count).await;
            }
            // println!("Data = {:?}",m);
            // let msg = ws_message::Text(data.to_string().into());
            // write.send(msg).await.unwrap();
            // println!("LTP :{:?}", data.to_string());
            // }
            Err(e) => {
                println!("Error with Price feed stream {:?}", e);
            }
        }
    }

    // LIVE PRICE FEED WEBSOCKET CONNECTION END

    // let candle_data_req = SmartConnect::new_candle_data(
    //     MarketDataExchange::NSE,
    //     "3045",
    //     aidynamics_trade::types::Interval::_5m,
    //     "2024-05-27 09:15",
    //     "2024-05-28 15:30",
    // )
    // .unwrap();
    // let cd = sc.candle_data(&candle_data_req).await.unwrap();
    // println!("Candle data: {:?}", cd);

    // for (symbol, open, high, low, close, volume) in &cd {
    //     println!("Symbol: {}, Open: {}, High: {}, Low: {}, Close: {}, Volume: {}", symbol, open, high, low, close, volume);
    // }
}

 */
