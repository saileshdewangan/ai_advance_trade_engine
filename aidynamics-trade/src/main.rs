#![allow(warnings)]
use aidynamics_trade::order_processor::order_processor::process_order;
use aidynamics_trade_utils::http::HttpClient;
use std::collections::{HashMap, HashSet};
use std::ops::Sub;
use std::time::Instant;
use std::{clone, string, thread, vec};
// use std::{array, f64::consts::E, fmt::write, intrinsics::mir::place, iter, str::FromStr};
use tokio::task;
use tracing::{error, info};

use aidynamics_trade::market::LtpDataReq;
use aidynamics_trade::order::CancelOrderReq;
use aidynamics_trade::trade_engine_processor::EngineProcessor;
use aidynamics_trade::trade_handler::Trade;
use aidynamics_trade::types::ExchangeType;
use aidynamics_trade::websocket::angel_one_websocket::{
    AngelOneWebSocketClient, SubscriptionBuilder, SubscriptionExchange, SubscriptionMode,
};
use aidynamics_trade::websocket::check_status::check_server_status;
// use aidynamics_trade::websocket::websocket_client::{Signal, WebSocketClient};
use aidynamics_trade::redis_utils::Signal;
use aidynamics_trade::ws::Message;
use aidynamics_trade::{end_points, order_processor, trade_handler, RedisUtils};
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
use redis::streams::StreamReadOptions;
use tokio::net::TcpStream;
use tokio::signal;
// use futures::channel::mpsc::Receiver;
use tokio::sync::broadcast;
use tokio::sync::mpsc::Receiver;

// use futures::channel::mpsc::Receiver;
use futures::{
    future::{join, join_all},
    SinkExt, StreamExt,
};
use reqwest::header::{self, HeaderMap};
use reqwest::Client;
use serde::{de::value, Deserialize, Serialize};
use serde_json::json;
use serde_json::{de::Read, Value};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{self, sleep, Duration};
use tokio_tungstenite::tungstenite::client;
use tokio_tungstenite::tungstenite::protocol::Message as ws_message;

// use websocket_client::WebSocketClient;
use aidynamics_trade::trade_engine::{self, EngineStatus, TradeStatus};
use tokio::sync::mpsc;
use tokio::sync::mpsc::Sender;
use tokio_tungstenite::{connect_async, WebSocketStream};

// use futures_util::{SinkExt, StreamExt};
// use tokio::sync::mpsc;
// use tokio_tungstenite::tungstenite::protocol::Message;
// use tokio_tungstenite::connect_async;
use aidynamics_trade::client_node::ClientNode;
use redis::{AsyncCommands, Commands, RedisResult};
use tracing_subscriber::Layer;
use tracing_subscriber::{fmt, layer::SubscriberExt, Registry};
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

#[derive(Serialize, Deserialize, Debug)]
struct User {
    name: String,
}

async fn integrate_firebase(
    firebase_url: &str,
    firebase_auth_key: &str,
    tx_main: mpsc::Sender<Signal>,
) {
    let mut firebase = Firebase::new(&firebase_url).unwrap().at("trades");
    // let firebase = Firebase::auth(&firebase_url, firebase_auth_key)
    //     .unwrap()
    //     .at("trades")
    //     .at("-OEmpyYF3oD3rZqoRNR_");
    println!("Integrating.... {:?}", firebase_url);
    let u = User {
        name: String::from("Pinku"),
    };
    firebase.update(&u).await;
    tokio::spawn(async move {
        let stream = firebase.with_realtime_events().unwrap();
        stream
            .listen(
                |event_type, data| {
                    println!("\nType: {:?} Data: {:?}", event_type, data);
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
    api_key: &String,
    client_code: &String,
    totp_token: &String,
    pin: &String,
) -> Result<()> {
    // Create an HTTP client
    let client = Client::new();

    // Define the URL to send the POST request to
    let url = format!("{}{}", api_url, "/secure/server/login"); // Example URL, replace with your actual endpoint

    // Define the payload for the POST request
    let payload = json!({
        "name": "RUST",
        "password":rust_password,
        "api_key":api_key,"client_code":client_code,"totp_token":totp_token,"pin":pin
    });

    // Send the POST request with JSON payload
    let response = client.post(url).json(&payload).send().await?;

    // Check if the request was successful

    match response.error_for_status() {
        Ok(res) => {
            let response_json: Value = res.json().await?;
            // println!("\n\nResponse: {}\n\n", response_json);

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

async fn get_instance_id(auth_token: &String, redis_client: &redis::Client) -> Result<()> {
    // Create an HTTP client
    let client = Client::new();

    // Define the URL to send the POST request to
    let api_url = format!(
        "{}{}",
        dotenv::var("API_URL").unwrap(),
        "/secure/instanceid"
    ); // Example URL, replace with your actual endpoint

    let response = client
        .post(api_url)
        .header(header::AUTHORIZATION, format!("Bearer {}", auth_token))
        .header(header::ACCEPT, "application/json")
        .header(header::CONTENT_TYPE, "application/json") // Attach the JSON body
        .send()
        .await?;

    match response.error_for_status() {
        Ok(res) => {
            let response_json: Value = res.json().await?;
            println!("\n\nResponse: {}\n\n", response_json);

            if let Some(success) = response_json.get("success") {
                if success == true {
                    // Get the "message" object from the response
                    if let Some(message) = response_json.get("message") {
                        // Fetch "authToken" from "message"
                        if let Some(instance_id) = message.get("instanceid") {
                            let mut con = redis_client
                                .get_multiplexed_async_connection()
                                .await
                                .unwrap();

                            let _: () = con
                                .set("Api:Instance", instance_id.to_string())
                                .await
                                .unwrap();
                        } else {
                            println!("\n\nError fetching authToken");
                        }
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

async fn InitializeApp(tx_redis: Sender<Signal>) {
    tx_redis.send(Signal::InitializeApp).await;
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

    let redis_conn_url = format!("redis://127.0.0.1/");

    redis::Client::open(redis_conn_url.as_str())
        .expect("Invalid connection URL")
        .get_connection()
        .expect("failed to connect to Redis")
}

fn create_channels() -> (
    mpsc::Sender<Signal>,
    mpsc::Receiver<Signal>,
    mpsc::Sender<Signal>,
    mpsc::Receiver<Signal>,
    tokio::sync::mpsc::Sender<Signal>,
    tokio::sync::mpsc::Receiver<Signal>,
    Arc<broadcast::Sender<Signal>>,
    broadcast::Receiver<Signal>,
    broadcast::Receiver<Signal>,
    broadcast::Receiver<Signal>,
    broadcast::Receiver<Signal>,
    tokio::sync::mpsc::Sender<Signal>,
    tokio::sync::mpsc::Receiver<Signal>,
) {
    // ... (rest of create_channels function) ...
    let (tx_redis, rx_redis) = mpsc::channel(100);
    let (tx_aows, rx_aows) = mpsc::channel(200);
    let (tx_main, rx_main) = tokio::sync::mpsc::channel::<Signal>(100);
    let (tx_broadcast, _) = broadcast::channel::<Signal>(100);
    let (tx_order_processor, rx_order_processor) = tokio::sync::mpsc::channel::<Signal>(100);

    let tx_broadcast_arc = Arc::new(tx_broadcast);

    let rx_strategy_1 = tx_broadcast_arc.subscribe();
    let rx_strategy_2 = tx_broadcast_arc.subscribe();
    let rx_strategy_3 = tx_broadcast_arc.subscribe();
    let rx_strategy_4 = tx_broadcast_arc.subscribe();

    (
        tx_redis,
        rx_redis,
        tx_aows,
        rx_aows,
        tx_main.clone(),
        rx_main,
        tx_broadcast_arc.clone(),
        rx_strategy_1,
        rx_strategy_2,
        rx_strategy_3,
        rx_strategy_4,
        tx_order_processor,
        rx_order_processor,
    )
}

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    tracing_subscriber::fmt::init(); // This enables logging to the console
                                     // let fmt_layer = fmt::layer().with_ansi(true); // Enable ANSI colors
                                     // let subscriber = Registry::default().with(fmt_layer);
                                     // tracing::subscriber::set_global_default(subscriber).expect("Failed to set global default subscriber");

    let api_key = dotenv::var("API_KEY").unwrap();
    let client_code = dotenv::var("CLIENT_CODE").unwrap();
    let pin = dotenv::var("PIN").unwrap();
    let totp_token = dotenv::var("OTP_TOKEN").unwrap();

    let firebase_url = dotenv::var("FIREBASE_URL").unwrap();
    let firebase_auth_key = dotenv::var("FIREBASE_AUTH_KEY").unwrap();

    let ws_url = dotenv::var("WS_URL").unwrap();
    let status_url = dotenv::var("STATUS_URL").unwrap();
    let rust_password = dotenv::var("NEW_RUST_PASSWORD").unwrap();

    // let redis_host = dotenv::var("REDIS_HOST").unwrap();
    // let redis_password = dotenv::var("REDIS_PASSWORD").unwrap();

    let api_url = dotenv::var("API_URL").unwrap();
    // BANKNIFTY = 26009,NIFTY = 26000, FINNIFTY = 26037

    let api_key_clone = api_key.clone();
    let client_code_clone = client_code.clone();
    let pin_clone = pin.clone();

    let mut auth_token = "".to_string();
    // let mut web_server_token = "";
    let mut angelone_tokens: Option<Value> = None;
    let mut jwt_token: String = "".to_string();
    let mut feed_token: String = "".to_string();
    let mut refresh_token = "".to_string();

    let redis_host_name = dotenv::var("REDIS_HOST").unwrap();
    let redis_password = dotenv::var("REDIS_PASSWORD").unwrap();
    let redis_conn_url = dotenv::var("REDIS_URL").unwrap();

    /// Creating a HashMap of Shared TradeEngine.
    /// Which value can only be chaned by main channel
    let redis_conn_url_clone = redis_conn_url.clone();

    // let redUt = RedisUtils::init(redis_conn_url, rx_redis, tx_main_clone).await;

    let client =
        redis::Client::open(redis_conn_url_clone.as_str()).expect("Invalid connection URL");

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
        &api_key_clone,
        &client_code_clone,
        &totp_token,
        &pin,
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

    let (
        tx_redis,
        mut rx_redis,
        tx_aows,
        mut rx_aows,
        tx_main,
        mut rx_main,
        tx_broadcast_arc,
        mut rx_strategy_1,
        mut rx_strategy_2,
        mut rx_strategy_3,
        mut rx_strategy_4,
        tx_order_processor,
        mut rx_order_processor,
    ) = create_channels();

    let tx_main_clone = tx_main.clone();
    let tx_main_firebase = tx_main.clone();

    // integrate_firebase(&firebase_url, &firebase_auth_key, tx_main_firebase).await;

    let redis_host_name = dotenv::var("REDIS_HOST").unwrap();
    let redis_password = dotenv::var("REDIS_PASSWORD").unwrap();
    // let redis_conn_url = format!("redis://:{}@{}", redis_password, redis_host_name);
    let redis_conn_url = format!("redis://127.0.0.1/");

    let redis_conn_url_clone = redis_conn_url.clone();

    // let redUt = RedisUtils::init(redis_conn_url, rx_redis, tx_main_clone, tx_brd_redis).await;
    let redUt = RedisUtils::init(
        redis_conn_url,
        rx_redis,
        tx_main_clone,
        tx_broadcast_arc.clone(),
    )
    .await;

    let client =
        redis::Client::open(redis_conn_url_clone.as_str()).expect("Invalid connection URL");

    let client_clone = client.clone();
    let tx_aows_clone = tx_aows.clone();

    let redis_client_clone = client.clone();

    /// PROCESS ALL TRADE HANDLERS BEGIN
    async fn receive_messages(
        rx_main: &mut Receiver<Signal>,
        tx_broadcast_clone: Arc<tokio::sync::broadcast::Sender<Signal>>,
        tx_aows: Sender<Signal>,
        tx_main: Sender<Signal>,
        tx_redis: Sender<Signal>,
        tx_order_processor: Sender<Signal>,
    ) {
        let mut client_nodes: HashMap<u32, ClientNode> = HashMap::new();
        //     let mut trade_handlers: Vec<TradeEngine> = Vec::new();
        while let Some(msg) = rx_main.recv().await {
            println!("\nReceived by main : {:?}", msg);
            let msg_clone = msg.clone();
            match msg_clone {
                Signal::NewTradeEngine(new_engine) => {
                    let engine = new_engine.clone();
                    let client_id = engine.client_id;
                    let trade_engine_id = engine.trade_engine_id;

                    if let Some(client_node) = client_nodes.get_mut(&client_id) {
                        // If the client exists, check if it already has the TradeEngine
                        if !client_node.trade_engines.contains_key(&trade_engine_id) {
                            client_node
                                .trade_engines
                                .insert(trade_engine_id, engine.clone());

                            client_node.active_trade_ids.insert(trade_engine_id);
                            client_node.handler_ids.remove(&trade_engine_id);
                        }
                    } else {
                        // If the client doesn't exist, create a new ClientNode
                        let mut trade_engines = HashMap::new();
                        trade_engines.insert(trade_engine_id, engine.clone());
                        let new_node = ClientNode {
                            client_id,
                            trade_engines,
                            rx_broadcast: tx_broadcast_clone.subscribe(),
                            tx_broadcast: tx_broadcast_clone.clone(),
                            tx_main: tx_main.clone(),
                            tx_redis: tx_redis.clone(),
                            tx_angelone_sender: tx_aows.clone(),
                            tx_order_processor: tx_order_processor.clone(),
                            active_trade_ids: {
                                let mut set = HashSet::new();
                                set.insert(trade_engine_id);
                                set
                            },
                            handler_ids: HashSet::new(),
                            strategy_to_process: engine.strategy.clone(),
                        };

                        client_nodes.insert(client_id, new_node);

                        let mut new_trade_engines = HashMap::new();
                        new_trade_engines.insert(trade_engine_id, engine.clone());
                        let mut new_client_node = ClientNode {
                            client_id,
                            trade_engines: new_trade_engines,
                            rx_broadcast: tx_broadcast_clone.subscribe(),
                            tx_broadcast: tx_broadcast_clone.clone(),
                            tx_main: tx_main.clone(),
                            tx_redis: tx_redis.clone(),
                            tx_angelone_sender: tx_aows.clone(),
                            tx_order_processor: tx_order_processor.clone(),
                            active_trade_ids: {
                                let mut set = HashSet::new();
                                set.insert(trade_engine_id);
                                set
                            },
                            handler_ids: HashSet::new(),
                            strategy_to_process: engine.strategy.clone(),
                        };

                        tokio::spawn(async move {
                            new_client_node.process_engine().await;
                        });
                    }
                    tx_broadcast_clone.send(msg);
                }

                // Forward all other messages to the broadcast channel
                _ => {
                    tx_broadcast_clone.send(msg);
                }
            }
        }
    };

    // Spawn another task to receive message
    let tx_main_cln = tx_main.clone();
    let tx_redis_clone = tx_redis.clone();
    let tx_broadcast_arc_clone = tx_broadcast_arc.clone();
    let tx_op_main = tx_order_processor.clone();
    let tx_aows_main = tx_aows.clone();
    let tx_redis_main = tx_redis.clone();
    tokio::spawn(async move {
        receive_messages(
            &mut rx_main,
            tx_broadcast_arc_clone,
            tx_aows_main.clone(),
            tx_main_cln,
            tx_redis_main,
            tx_op_main,
        )
        .await;
        // sleep(Duration::from_secs(5));
    });

    // Spawn another task for startegy 2
    let tx_main_2 = tx_main.clone();
    let tx_redis_2 = tx_redis.clone();
    let client_2 = client.clone();
    let tx_aows_2 = tx_aows.clone();
    let tx_op_clone_2 = tx_order_processor.clone();
    let trade_handlers_2: HashMap<u32, TradeEngine> = HashMap::new();
    let active_trade_ids_2 = HashSet::new();
    let handler_ids_2: HashSet<u32> = HashSet::new();

    let mut engine_processor_2 = EngineProcessor::new(
        Strategy::RangeBreak,
        tx_broadcast_arc.clone(),
        tx_main_2,
        tx_redis_2,
        client_2,
        tx_aows_2,
        tx_op_clone_2,
        trade_handlers_2,
        active_trade_ids_2,
        handler_ids_2,
    );

    tokio::spawn(async move {
        engine_processor_2.process_engine(rx_strategy_2).await;
    });

    // Spawn another task for startegy 3
    let tx_main_3 = tx_main.clone();
    let tx_redis_3 = tx_redis.clone();
    let client_3 = client.clone();
    let tx_aows_3 = tx_aows.clone();
    let tx_op_clone_3 = tx_order_processor.clone();
    let trade_handlers_3: HashMap<u32, TradeEngine> = HashMap::new();
    let active_trade_ids_3 = HashSet::new();
    let handler_ids_3: HashSet<u32> = HashSet::new();

    let mut engine_processor_3 = EngineProcessor::new(
        Strategy::MomentumShift,
        tx_broadcast_arc.clone(),
        tx_main_3,
        tx_redis_3,
        client_3,
        tx_aows_3,
        tx_op_clone_3,
        trade_handlers_3,
        active_trade_ids_3,
        handler_ids_3,
    );

    tokio::spawn(async move {
        engine_processor_3.process_engine(rx_strategy_3).await;
    });

    // Spawn another task for startegy 4
    let tx_main_4 = tx_main.clone();
    let tx_redis_4 = tx_redis.clone();
    let client_4 = client.clone();
    let tx_aows_4 = tx_aows.clone();
    let tx_op_clone_4 = tx_order_processor.clone();
    let trade_handlers_4: HashMap<u32, TradeEngine> = HashMap::new();
    let active_trade_ids_4 = HashSet::new();
    let handler_ids_4: HashSet<u32> = HashSet::new();

    let mut engine_processor_4 = EngineProcessor::new(
        Strategy::DynamicChannel,
        tx_broadcast_arc.clone(),
        tx_main_4,
        tx_redis_4,
        client_4,
        tx_aows_4,
        tx_op_clone_4,
        trade_handlers_4,
        active_trade_ids_4,
        handler_ids_4,
    );

    tokio::spawn(async move {
        engine_processor_4.process_engine(rx_strategy_4).await;
    });

    let tx_main_order = tx_main.clone();
    let tx_redis_order = tx_redis.clone();
    let tx_broadcast_main = tx_broadcast_arc.clone();

    tokio::spawn(async move {
        process_order(
            &mut rx_order_processor,
            tx_main_order,
            tx_broadcast_arc.clone(),
            tx_redis_order,
        )
        .await;
    });

    let feedToken = feed_token_clone.clone();
    let clientCode = client_code.clone();
    let apiKey = api_key.clone();

    let wsUrl = format!(
        "{}clientCode={}&feedToken={}&apiKey={}",
        "wss://smartapisocket.angelone.in/smart-stream?", clientCode, feedToken, apiKey
    );
    let aows = AngelOneWebSocketClient::connect(wsUrl, rx_aows, tx_broadcast_main).await;

    let tx_aows_clone = tx_aows.clone();

    let subscription = SubscriptionBuilder::new("abcde12345")
        .mode(SubscriptionMode::Ltp)
        // .subscribe(SubscriptionExchange::MCXFO, vec!["429116".into()])
        .subscribe(
            SubscriptionExchange::NSECM,
            vec!["26009".into(), "26000".into(), "26037".into()],
        )
        .subscribe(SubscriptionExchange::BSECM, vec!["19000".into()])
        .build();

    tx_aows_clone
        .send(Signal::Subscribe(subscription))
        .await
        .unwrap();

    // LIVE PRICE FEED WEBSOCKET CONNECTION END

    let tx_redis_init = tx_redis.clone();

    InitializeApp(tx_redis_init).await;

    loop {
        // get_instance_id(&auth_token, &redis_client_clone).await;
        std::thread::sleep(std::time::Duration::from_secs(5));
        // println!("Main function is still running...");
    }
}
