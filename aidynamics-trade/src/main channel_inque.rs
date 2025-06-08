#![allow(warnings)]
use core::task;
use std::collections::HashMap;
use std::ops::Sub;
use std::time::Instant;
use std::{clone, string, thread, vec};
// use std::{array, f64::consts::E, fmt::write, intrinsics::mir::place, iter, str::FromStr};

use aidynamics_trade::market::LtpDataReq;
use aidynamics_trade::order::CancelOrderReq;
use aidynamics_trade::trade_engine_processor::process_engine;
use aidynamics_trade::trade_handler::Trade;
use aidynamics_trade::types::ExchangeType;
use aidynamics_trade::websocket::angel_one_websocket::{
    AngelOneWebSocketClient, SubscriptionBuilder, SubscriptionExchange, SubscriptionMode,
};
use aidynamics_trade::websocket::check_status::check_server_status;
// use aidynamics_trade::websocket::websocket_client::{Signal, WebSocketClient};
use aidynamics_trade::redis_utils::Signal;
use aidynamics_trade::{end_points, trade_handler, RedisUtils};
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
use tokio::sync::broadcast;

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
use redis::{AsyncCommands, Commands};
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

/// Instrument types
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub enum Instrument {
    /// Nifty index
    NIFTY,
    /// Banknifty index
    BANKNIFTY,
    /// Finnifty index
    FINNIFTY,
    /// Any other stock || option || future
    Other,
}

impl ToString for Instrument {
    fn to_string(&self) -> String {
        match self {
            Instrument::NIFTY => "NIFTY".to_string(),
            Instrument::BANKNIFTY => "BANKNIFTY".to_string(),
            Instrument::FINNIFTY => "FINNIFTY".to_string(),
            Instrument::Other => "Other".to_string(),
        }
    }
}

fn get_instrument(token: &str) -> Instrument {
    match token {
        "26000" => Instrument::NIFTY,
        "26009" => Instrument::BANKNIFTY,
        "26037" => Instrument::FINNIFTY,
        _ => Instrument::Other,
    }
}

impl Instrument {
    /// Determines if the instrument is an index
    fn is_index(&self) -> bool {
        matches!(
            self,
            Instrument::NIFTY | Instrument::BANKNIFTY | Instrument::FINNIFTY
        )
    }
}

async fn integrate_firebase(
    firebase_url: &str,
    firebase_auth_key: &str,
    tx_main: mpsc::Sender<Signal>,
) {
    let firebase = Firebase::new(&firebase_url).unwrap().at("trades");
    // let firebase = Firebase::auth(&firebase_url, firebase_auth_key)
    //    .unwrap()
    //    .at("trades");
    tokio::spawn(async move {
        let stream = firebase.with_realtime_events().unwrap();
        // stream
        //     .listen(
        //         |event_type, data| {
        //             println!("\nType: {:?} Data: {:?}", event_type, data);
        //         },
        //         |err| println!("{:?}", err),
        //         false,
        //     )
        //     .await;
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

    // if response.status().is_success() {

    // } else {
    //     eprintln!("Failed to send POST request. Status: {}", response.status());
    // }
    Ok(())
}

async fn handle_buy_signal(handler: &mut TradeEngine, ltp: f32, tx_redis: Sender<Signal>) {
    if handler.transaction_type == TransactionType::BUY && ltp > handler.trigger_price {
        handler.execute_trade(tx_redis).await;
    }
}

async fn handle_sell_signal(handler: &mut TradeEngine, ltp: f32, tx_redis: Sender<Signal>) {
    if handler.transaction_type == TransactionType::SELL && ltp < handler.trigger_price {
        handler.execute_trade(tx_redis.clone()).await;
    }
}

fn connect() -> redis::Connection {
    //format - host:port
    let redis_host_name = dotenv::var("REDIS_HOST").unwrap();
    let redis_password = dotenv::var("REDIS_PASSWORD").unwrap();

    //if Redis server needs secure connection
    let uri_scheme = match dotenv::var("IS_TLS") {
        Ok(_) => "rediss",
        Err(_) => "redis",
    };

    // let redis_conn_url = format!("redis://:{}@{}", redis_password, redis_host_name);
    let redis_conn_url = format!("redis://127.0.0.1/");

    // let redis_conn_url = format!("{}@{}", redis_password, redis_host_name);
    //println!("{}", redis_conn_url);

    redis::Client::open(redis_conn_url.as_str())
        .expect("Invalid connection URL")
        .get_connection()
        .expect("failed to connect to Redis")
}

#[tokio::main]
async fn main() {
    let api_key = dotenv::var("API_KEY").unwrap();
    let client_code = dotenv::var("CLIENT_CODE").unwrap();
    let pin = dotenv::var("PIN").unwrap();
    let otp_token = dotenv::var("OTP_TOKEN").unwrap();

    let firebase_url = dotenv::var("FIREBASE_URL").unwrap();
    let firebase_auth_key = dotenv::var("FIREBASE_AUTH_KEY").unwrap();

    let ws_url = dotenv::var("WS_URL").unwrap();
    let status_url = dotenv::var("STATUS_URL").unwrap();
    let rust_password = dotenv::var("NEW_RUST_PASSWORD").unwrap();

    // let redis_host = dotenv::var("REDIS_HOST").unwrap();
    // let redis_password = dotenv::var("REDIS_PASSWORD").unwrap();

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

    // Create an mpsc channel that will be used to publish messages to the redis server
    let (tx_redis, mut rx_redis) = mpsc::channel(100);

    let (tx_aows, mut rx_aows) = mpsc::channel(100);

    // Create an mpsc channel so you can send messages from the main thread and other tasks
    let (tx_main, mut rx_main) = tokio::sync::mpsc::channel::<Signal>(100);

    let (tx_broadcast, mut rx_broadcast) = broadcast::channel::<Signal>(100);

    /// Strategy 1 = Stochastic RSI
    let (tx_strategy_1, mut rx_strategy_1) = tokio::sync::mpsc::channel::<Signal>(100);

    /// Strategy 2 = MACD + RSI
    let (tx_strategy_2, mut rx_strategy_2) = tokio::sync::mpsc::channel::<Signal>(100);

    /// Strategy 3 = 5EMA
    let (tx_strategy_3, mut rx_strategy_3) = tokio::sync::mpsc::channel::<Signal>(100);

    /// Strategy 4 = Bollinger Bands
    let (tx_strategy_4, mut rx_strategy_4) = tokio::sync::mpsc::channel::<Signal>(100);

    let tx_main_clone = tx_main.clone();
    let tx_main_firebase = tx_main.clone();

    // loop {
    //     if check_server_status(&status_url.as_str(), Duration::from_secs(2)).await {
    //         break;
    //     } else {
    //         println!("\n\nServer is offline, retrying in 5 seconds...");
    //         sleep(Duration::from_secs(5)).await;
    //     }
    // }

    // Start the WebSocket client and pass the `rx` receiver to receive messages
    // WebSocketClient::connect(ws_url.clone(), rx_redis, tx_main_clone.clone(), auth_token).await;

    integrate_firebase(&firebase_url, &firebase_auth_key, tx_main_firebase).await;

    let redis_host_name = dotenv::var("REDIS_HOST").unwrap();
    let redis_password = dotenv::var("REDIS_PASSWORD").unwrap();
    // let redis_conn_url = format!("redis://:{}@{}", redis_password, redis_host_name);
    let redis_conn_url = format!("redis://127.0.0.1/");

    let redis_conn_url_clone = redis_conn_url.clone();

    let redUt = RedisUtils::init(redis_conn_url, rx_redis, tx_main_clone).await;

    let client =
        redis::Client::open(redis_conn_url_clone.as_str()).expect("Invalid connection URL");

    let client_clone = client.clone();
    let tx_aows_clone = tx_aows.clone();

    let tx_strategy_1_clone = tx_strategy_1.clone();
    let tx_strategy_2_clone = tx_strategy_2.clone();
    let tx_strategy_3_clone = tx_strategy_3.clone();
    let tx_strategy_4_clone = tx_strategy_4.clone();

    /// PROCESS ALL TRADE HANDLERS BEGIN
    async fn receive_messages(
        rx_main: &mut Receiver<Signal>,
        tx_strategy_1: Sender<Signal>,
        tx_strategy_2: Sender<Signal>,
        tx_strategy_3: Sender<Signal>,
        tx_strategy_4: Sender<Signal>,
    ) {
        //     let mut trade_handlers: Vec<TradeEngine> = Vec::new();

        while let Some(msg) = rx_main.recv().await {
            // tx_strategy_1.send(msg.clone()).await;
            // tx_strategy_2.send(msg.clone()).await;
            // tx_strategy_3.send(msg.clone()).await;
            // tx_strategy_4.send(msg.clone()).await;

            if let Err(e) = tx_strategy_1.send(msg.clone()).await {
                eprintln!("Error forwarding to strategy 1: {:?}", e);
            }
            if let Err(e) = tx_strategy_2.send(msg.clone()).await {
                eprintln!("Error forwarding to strategy 2: {:?}", e);
            }
            if let Err(e) = tx_strategy_3.send(msg.clone()).await {
                eprintln!("Error forwarding to strategy 3: {:?}", e);
            }
            if let Err(e) = tx_strategy_4.send(msg.clone()).await {
                eprintln!("Error forwarding to strategy 4: {:?}", e);
            }
        }
    };

    // async fn receive_messages(
    //     rx_main: &mut Receiver<Signal>,
    //     tx_main_clone: Sender<Signal>,
    //     tx_redis_clone: Sender<Signal>,
    //     redis_client: redis::Client,
    //     tx_angelone_sender: Sender<Signal>,
    // ) {
    //     let mut trade_handlers: Vec<TradeEngine> = Vec::new();

    //     while let Some(msg) = rx_main.recv().await {
    //         // Process the message
    //         // println!("\n\nMessage is {:?}", msg);
    //         match msg {
    //             Signal::PriceFeed { token, ltp } => {
    //                 let instrument = get_instrument(&token);
    //                 for handler in trade_handlers.iter_mut() {
    //                     match instrument {
    //                         Instrument::NIFTY | Instrument::BANKNIFTY | Instrument::FINNIFTY => {
    //                             if handler.trade_status == TradeStatus::Pending
    //                                 && handler.symbol == instrument.to_string()
    //                             {
    //                                 // println!(
    //                                 //     "Buy {:?} Trg. price : {:?}, Ltp : {:?}",
    //                                 //     handler.symbol, handler.trigger_price, ltp
    //                                 // );

    //                                 match handler.transaction_type {
    //                                     TransactionType::BUY => {
    //                                         handle_buy_signal(handler, ltp, tx_redis_clone.clone())
    //                                             .await
    //                                     }
    //                                     TransactionType::SELL => {
    //                                         handle_sell_signal(handler, ltp, tx_redis_clone.clone())
    //                                             .await
    //                                     }
    //                                     _ => {}
    //                                 }
    //                             }
    //                         }
    //                         Instrument::Other => {
    //                             if handler.symbol_token == token
    //                                 && handler.trade_status == TradeStatus::Open
    //                             {
    //                                 if handler.transaction_type == TransactionType::BUY {
    //                                     println!(
    //                                         "Buy Pnl : {:?}",
    //                                         (ltp - handler.trade_entry_price)
    //                                             * handler.quantity as f32
    //                                     );
    //                                     if handler.sl != 0.0
    //                                         && ltp <= (handler.trade_entry_price - handler.sl)
    //                                     {
    //                                         handler
    //                                             .squareoff_trade(&tx_redis_clone.clone(), false)
    //                                             .await;
    //                                         println!("\n\nBuy square off trade");
    //                                     }
    //                                 } else {
    //                                     println!(
    //                                         "Sell Pnl : {:?}",
    //                                         (handler.trade_entry_price - ltp)
    //                                             * handler.quantity as f32
    //                                     );

    //                                     if handler.sl != 0.0
    //                                         && ltp >= (handler.sl - handler.trade_entry_price)
    //                                     {
    //                                         handler
    //                                             .squareoff_trade(&tx_redis_clone.clone(), false)
    //                                             .await;
    //                                         println!("\n\nSell square off trade");
    //                                     }
    //                                 }
    //                             }
    //                         }
    //                     }
    //                 }
    //             }
    //             Signal::OpenNewTrade(engine) => {
    //                 println!("\n\nMain channel : OpenNewTrade");
    //                 if let Some(existing_engine) = trade_handlers.iter_mut().find(|existing| {
    //                     existing.strategy == engine.strategy
    //                         && existing.symbol == engine.symbol
    //                         && existing.executed_trades < existing.max_trades
    //                 }) {
    //                     existing_engine.trade_status = TradeStatus::Pending;
    //                     println!("New trade pushed {:?}", engine);
    //                     existing_engine.exchange_type = engine.exchange_type;
    //                     existing_engine.trigger_price = engine.trigger_price;

    //                     match existing_engine.transaction_type {
    //                         TransactionType::BUY => {
    //                             existing_engine.symbol_token = engine.buyer_symbol_token;
    //                             existing_engine.trading_symbol = engine.buyer_trading_symbol;
    //                         }
    //                         TransactionType::SELL => {
    //                             existing_engine.symbol_token = engine.seller_symbol_token;
    //                             existing_engine.trading_symbol = engine.seller_trading_symbol;
    //                         }
    //                     }

    //                     println!(
    //                         "Found with trigger price {:?}",
    //                         existing_engine.trigger_price
    //                     );

    //                     // Call the method to prepare entry
    //                     existing_engine.prepare_entry_req().await;

    //                     existing_engine.prepare_exit_req().await;

    //                     let subscription = SubscriptionBuilder::new("abcde12345")
    //                         .mode(SubscriptionMode::Ltp)
    //                         .subscribe(
    //                             SubscriptionExchange::NSEFO,
    //                             vec![existing_engine.symbol_token.as_str()],
    //                         )
    //                         .build();

    //                     tx_angelone_sender
    //                         .send(Signal::Subscribe(subscription))
    //                         .await
    //                         .unwrap();

    //                     println!("Prepared entry req : {:?}", existing_engine.entry_req);
    //                 } else {
    //                     println!(
    //                         "No TradeEngine with corresponding symbol {:?} and strategy {:?}",
    //                         engine.symbol, engine.strategy
    //                     );
    //                 }
    //             }
    //             Signal::OrderPlaced(resp) => {
    //                 if let Some(handler) = trade_handlers.iter_mut().find(|handler| {
    //                     handler.trade_status == TradeStatus::Confirming
    //                         && handler.trade_engine_id == resp.trade_engine_id // Check for matching trade_id
    //                 }) {
    //                     // handler.wait = false;
    //                     // handler.pause = false;
    //                     handler.trade_status = TradeStatus::Open;
    //                     handler.trade_entry_price = resp.price;
    //                     handler.trade_id = resp.trade_id;
    //                     handler.executed_trades += 1;
    //                     handler.prepare_exit_req();
    //                 }
    //             }
    //             Signal::CancelOrder {
    //                 symbol,
    //                 strategy,
    //                 transaction_type,
    //                 trigger_price,
    //             } => {
    //                 if let Some(handler) = trade_handlers.iter_mut().find(|handler| {
    //                     handler.trade_status == TradeStatus::Pending
    //                         && handler.symbol == symbol
    //                         && handler.strategy == strategy
    //                         && handler.transaction_type == transaction_type
    //                         && handler.trigger_price == trigger_price
    //                 }) {
    //                     handler.trade_status = TradeStatus::Closed;
    //                     handler.exchange_type = ExchangeType::NFO;
    //                     handler.symbol_token = String::from("");
    //                     handler.trigger_price = 0.0;
    //                     handler.trading_symbol = String::from("");

    //                     println!(
    //                         "Open order cancelled ? Trade Engine Id -> {:?}",
    //                         handler.trade_engine_id
    //                     );
    //                 }
    //             }
    //             Signal::NewTradeEngine(engine) => {
    //                 // Signal::AddTradeEngine(engine) => {
    //                 println!("\n\nReceived signal on main channel");
    //                 let mut new_engine = engine.clone(); //TradeEngine::create_trade_engine(engine).await.unwrap();

    //                 if !trade_handlers
    //                     .iter()
    //                     .any(|engine| engine.trade_engine_id == new_engine.trade_engine_id)
    //                 {
    //                     if new_engine.trade_status == TradeStatus::Closed {
    //                         let trans_type = if new_engine.transaction_type == TransactionType::BUY
    //                         {
    //                             TransactionType::SELL
    //                         } else {
    //                             TransactionType::BUY
    //                         };
    //                         println!("\nnew e = {:?}", new_engine);
    //                         let exit_req = PlaceOrderReq::new(
    //                             new_engine.trading_symbol.clone(),
    //                             new_engine.symbol_token.clone(),
    //                             trans_type,
    //                         )
    //                         .exchange(new_engine.exchange_type)
    //                         .quantity(new_engine.quantity)
    //                         .product_type(ProductType::IntraDay);
    //                         new_engine.exit_req = Some(exit_req);
    //                     }

    //                     println!(
    //                         "\n\nNew Engine trade Status = {:?} exit req {:?}",
    //                         new_engine.trade_status, new_engine.exit_req
    //                     );

    //                     trade_handlers.push(new_engine);
    //                     println!(
    //                         "\n\nAdded : No of trade engines = {:?}",
    //                         trade_handlers.len()
    //                     );
    //                 } else {
    //                     println!(
    //                         "TradeEngine with trade_engine_id {} already exists.",
    //                         new_engine.trade_engine_id
    //                     );
    //                 }

    //                 // trade_handlers.push(new_engine);
    //                 // match add_trade_engine(config, tx_redis_clone.clone()).await {
    //                 //     Ok(trade_handler) => {
    //                 //         trade_handlers.push(trade_handler);
    //                 //     }
    //                 //     Err(e) => {
    //                 //         println!("\n\nError add trade handler... in main channel {:?}", e);
    //                 //     }
    //                 // }
    //                 println!("\n\nNo of trade engine == {:?}", trade_handlers.len());
    //             }
    //             Signal::RemoveTradeEngine(trade_engine_id) => {
    //                 // Find and remove the trade engine with the matching ID from the vector
    //                 trade_handlers.retain(|engine| engine.trade_engine_id != trade_engine_id);
    //                 println!(
    //                     "\n\nRemoved : No of trades engines = {:?}",
    //                     trade_handlers.len()
    //                 );
    //             }
    //             Signal::TradeEngineExist { client_id, status } => {
    //                 if let Some(trd_engine) = trade_handlers
    //                     .iter()
    //                     .find(|handler| handler.client_id == client_id)
    //                 {
    //                     tx_redis_clone
    //                         .send(Signal::TradeEngineExist {
    //                             client_id,
    //                             status: true,
    //                         })
    //                         .await;
    //                 } else {
    //                     tx_redis_clone
    //                         .send(Signal::TradeEngineExist {
    //                             client_id,
    //                             status: false,
    //                         })
    //                         .await;
    //                 }
    //             }
    //             Signal::UpdateTradeEngine {
    //                 trade_engine_id,
    //                 client_id,
    //                 config,
    //             } => {
    //                 if let Some(handler) = trade_handlers.iter_mut().find(|handler| {
    //                     handler.client_id == client_id
    //                         && handler.trade_status == TradeStatus::Closed
    //                         && handler.trade_engine_id == trade_engine_id // Check for matching trade_id
    //                 }) {
    //                     handler.max_loss = config.max_loss;
    //                     handler.max_price = config.max_price;
    //                     handler.max_trades = config.max_trades;
    //                     handler.quantity = config.quantity;
    //                     handler.sl = config.sl;
    //                     handler.strategy = config.strategy;
    //                     handler.target = config.target;
    //                     handler.trailing_sl = config.trailing_sl;
    //                     handler.transaction_type = config.transaction_type;
    //                 }
    //             }
    //             Signal::SquareOffTradeReq {
    //                 instance_id,
    //                 client_id,
    //                 trade_id,
    //                 remove_trade_engine,
    //             } => {
    //                 let mut found = false;

    //                 for handler in trade_handlers.iter_mut() {
    //                     if handler.client_id == client_id
    //                         && handler.trade_status == TradeStatus::Open
    //                         && handler.trade_id == trade_id
    //                     // Check for matching trade_id
    //                     {
    //                         if let Some(req) = &handler.exit_req {
    //                             found = true;
    //                             handler
    //                                 .squareoff_trade(&tx_redis_clone.clone(), false)
    //                                 .await;

    //                             // let instance_id_clone = instance_id.clone();

    //                             // tx_redis_clone
    //                             //     .send(Signal::SquaredOff {
    //                             //         instance_id: instance_id_clone,
    //                             //         trade_id: handler.trade_id,
    //                             //         // order_req: req.clone(),
    //                             //         client_id: handler.client_id,
    //                             //     })
    //                             //     .await;
    //                         }
    //                     }
    //                 }

    //                 let instance_id_clone = instance_id.clone();
    //                 let mut con = redis_client
    //                     .get_multiplexed_async_connection()
    //                     .await
    //                     .unwrap();
    //                 if !found {
    //                     println!(
    //                         "Handler not found for client_id = {:?} and trade_id = {:?}",
    //                         client_id, trade_id
    //                     );

    //                     // tx_redis_clone
    //                     //     .send(Signal::SquareOffReject {
    //                     //         instance_id: instance_id_clone,
    //                     //         trade_id,
    //                     //         error: "No trade found".to_string(),
    //                     //         message: "No trade found for this client".to_string(),
    //                     //     })
    //                     //     .await;

    //                     let _: () = con
    //                         .set(
    //                             format!("squareoffReq:{}", trade_id.to_string()), // Convert u32 to String for key
    //                             json!({
    //                                 "res":"reject",
    //                                 "instance_id": instance_id_clone.as_str(),
    //                                 "trade_id": trade_id.to_string(), // Ensure `trade_id` is converted to String
    //                                 "client_id": client_id.to_string(),
    //                                 "error": "No trade found".to_string(),
    //                                 "message":"No trade found for this client"
    //                             })
    //                             .to_string(), // Serialize JSON to String before storing in Redis
    //                         )
    //                         .await
    //                         .unwrap();
    //                 } else {
    //                     let _: () = con
    //                         .set(
    //                             format!("squareoffReq:{}", trade_id.to_string()), // Convert u32 to String for key
    //                             json!({
    //                                 "res":"acknowledge",
    //                                 "instance_id": instance_id_clone.as_str(),
    //                                 "trade_id": trade_id.to_string(), // Ensure `trade_id` is converted to String
    //                                 "client_id": client_id.to_string(),
    //                                 "message":"Trade squared off successfully."
    //                             })
    //                             .to_string(), // Serialize JSON to String before storing in Redis
    //                         )
    //                         .await
    //                         .unwrap();
    //                 }
    //             }
    //             Signal::Disconnect(client_id) => {
    //                 for handler in trade_handlers.iter_mut() {
    //                     println!(
    //                         "\nHander client id = {:?}, trade status = {:?}",
    //                         handler.client_id, handler.trade_status
    //                     );
    //                     if handler.client_id == client_id
    //                         && handler.trade_status == TradeStatus::Open
    //                     {
    //                         if let Some(req) = &handler.exit_req {
    //                             handler.squareoff_trade(&tx_redis_clone.clone(), true).await;
    //                         }
    //                     } else {
    //                         println!("Handler client id -> {:?}, param client id -> {:?}, Status -> {:?}",handler.client_id,client_id,handler.trade_status);
    //                     }
    //                 }
    //                 // Retain only trade engines that do not match the `client_id`
    //                 // trade_handlers.retain(|engine| engine.client_id == client_id);

    //                 let mut i = 0;
    //                 while i < trade_handlers.len() {
    //                     if trade_handlers[i].client_id == client_id {
    //                         trade_handlers.remove(i); // Remove matching element
    //                     } else {
    //                         i += 1; // Only increment if not removed
    //                     }
    //                 }

    //                 println!("\nHandlers count -> {:?}", trade_handlers.len());
    //             }
    //             Signal::WsConnectionClosed => {
    //                 // let mut ws_ready = ws_ready_clone.lock().unwrap();
    //                 // *ws_ready = false;
    //             }
    //             _ => {
    //                 println!("\n\nOther signal received inside.... {:?}", msg);
    //             }
    //         }
    //     }
    // }

    /// PROCESS TRADE HANDLERS END
    // Spawn another task to receive message
    let tx_main_cln = tx_main.clone();
    let tx_redis_clone = tx_redis.clone();
    tokio::spawn(async move {
        receive_messages(
            &mut rx_main,
            tx_strategy_1_clone,
            tx_strategy_2_clone,
            tx_strategy_3_clone,
            tx_strategy_4_clone,
        )
        .await;
        // sleep(Duration::from_secs(5));
    });

    // Spawn another task for startegy 1
    let tx_main_1 = tx_main.clone();
    let tx_redis_1 = tx_redis.clone();
    let client_1 = client.clone();
    let tx_aows_1 = tx_aows.clone();

    tokio::spawn(async move {
        process_engine(
            Strategy::Momentum,
            &mut rx_strategy_1,
            tx_main_1,
            tx_redis_1,
            client_1,
            tx_aows_1,
        )
        .await;
        // sleep(Duration::from_secs(5));
    });

    // Spawn another task for startegy 2
    let tx_main_2 = tx_main.clone();
    let tx_redis_2 = tx_redis.clone();
    let client_2 = client.clone();
    let tx_aows_2 = tx_aows.clone();

    tokio::spawn(async move {
        process_engine(
            Strategy::RangeBreak,
            &mut rx_strategy_2,
            tx_main_2,
            tx_redis_2,
            client_2,
            tx_aows_2,
        )
        .await;
        // sleep(Duration::from_secs(5));
    });

    // Spawn another task for startegy 3
    let tx_main_3 = tx_main.clone();
    let tx_redis_3 = tx_redis.clone();
    let client_3 = client.clone();
    let tx_aows_3 = tx_aows.clone();

    tokio::spawn(async move {
        process_engine(
            Strategy::MomentumShift,
            &mut rx_strategy_3,
            tx_main_3,
            tx_redis_3,
            client_3,
            tx_aows_3,
        )
        .await;
        // sleep(Duration::from_secs(5));
    });

    // Spawn another task for startegy 4
    let tx_main_4 = tx_main.clone();
    let tx_redis_4 = tx_redis.clone();
    let client_4 = client.clone();
    let tx_aows_4 = tx_aows.clone();

    tokio::spawn(async move {
        process_engine(
            Strategy::DynamicChannel,
            &mut rx_strategy_4,
            tx_main_4,
            tx_redis_4,
            client_4,
            tx_aows_4,
        )
        .await;
        // sleep(Duration::from_secs(5));
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
        //         WebSocketClient::connect(ws_url, rx_redis, tx_main_clone, auth_token).await;
        //     } else {
        //         println!("\n\nServer is offline, retrying in 5 seconds...");
        //         sleep(Duration::from_secs(5)).await;
        //     }
        // }

        std::thread::sleep(std::time::Duration::from_secs(5));
        // println!("Main function is still running...");
    }
}
