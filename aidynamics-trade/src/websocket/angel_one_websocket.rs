use byteorder::{ByteOrder, LittleEndian};
//use bytes::BytesMut;
use futures_util::{SinkExt, StreamExt};
use serde_repr::{Deserialize_repr, Serialize_repr};
use tokio::time::sleep;
// use serde_json::json;
use std::str;
use std::{sync::Arc, time::Instant};
use tokio::sync::mpsc;
use tokio::{sync::Mutex, time::Duration};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

// use crate::types::ExchangeType;

use crate::redis_utils::Signal;

#[derive(Debug)]
struct Ltp {
    subscription_mode: u8,
    ltp: f64,
    exchange_type: u8,
    token: String,
    sequence_number: u64,
    exchange_timestamp: u64,
    exchange_date: String,
}

#[derive(Debug)]
struct Quote {
    subscription_mode: u8,
    ltp: f64,
    exchange_type: u8,
    token: String,
    sequence_number: u64,
    exchange_timestamp: u64,
    exchange_date: String,
    last_traded_quantity: u64,
    average_traded_price: u64,
    volume_traded: u64,
}

// async fn parse_ltp(array_buffer: &[u8]) -> Ltp {
//     let subscription_mode = array_buffer[0];
//     let ltp = i32::from_le_bytes(array_buffer[43..47].try_into().unwrap()) as f64 / 100.0;
//     let exchange_type = array_buffer[1];
//     let token = str::from_utf8(&array_buffer[2..27])
//         .unwrap()
//         .trim_end_matches('\u{0}')
//         .to_string();
//     let sequence_number = u64::from_le_bytes(array_buffer[27..35].try_into().unwrap());
//     let exchange_timestamp = u64::from_le_bytes(array_buffer[35..43].try_into().unwrap());
//     let exchange_date = format!("{:?}", Instant::now());

//     Ltp {
//         subscription_mode,
//         ltp,
//         exchange_type,
//         token,
//         sequence_number,
//         exchange_timestamp,
//         exchange_date,
//     }
// }

async fn parse_quote(array_buffer: &[u8]) -> Quote {
    let subscription_mode = array_buffer[0];
    let ltp = i32::from_le_bytes(array_buffer[43..47].try_into().unwrap()) as f64 / 100.0;
    let exchange_type = array_buffer[1];
    let token = str::from_utf8(&array_buffer[2..27])
        .unwrap()
        .trim_end_matches('\u{0}')
        .to_string();
    let sequence_number = u64::from_le_bytes(array_buffer[27..35].try_into().unwrap());
    let exchange_timestamp = u64::from_le_bytes(array_buffer[35..43].try_into().unwrap());
    let exchange_date = format!("{:?}", Instant::now());
    let last_traded_quantity = u64::from_le_bytes(array_buffer[51..59].try_into().unwrap());
    let average_traded_price = u64::from_le_bytes(array_buffer[59..67].try_into().unwrap());
    let volume_traded = u64::from_le_bytes(array_buffer[67..75].try_into().unwrap());

    Quote {
        subscription_mode,
        ltp,
        exchange_type,
        token,
        sequence_number,
        exchange_timestamp,
        exchange_date,
        last_traded_quantity,
        average_traded_price,
        volume_traded,
    }
}

// async fn connect_angelone_websocket(
//     client_code: &str,
//     feed_token: &str,
//     api_key: &str,
//     on_message: impl Fn(Ltp),
// ) {
//     let ws_url = format!(
//         "wss://smartapisocket.angelone.in/smart-stream?clientCode={}&feedToken={}&apiKey={}",
//         client_code, feed_token, api_key
//     );

//     tokio::task::spawn(async move {
//         // Connect to WebSocket
//         let (ws_stream, _) = connect_async(&ws_url).await.expect("Failed to connect");
//         let (mut write, mut read) = ws_stream.split();

//         // Channel for the heartbeat
//         let (tx, mut rx) = mpsc::channel::<()>(1);
//         let heartbeat_interval = IntervalStream::new(time::interval(Duration::from_secs(30)));

//         let mut write = Arc::new(Mutex::new(write));

//         // Task to send heartbeat
//         tokio::spawn(async move {
//             tokio::pin!(heartbeat_interval);

//             while let Some(_) = heartbeat_interval.next().await {
//                 let mut write_locked = write.lock().await;
//                 if let Err(e) = write_locked.send(Message::Ping(vec![])).await {
//                     eprintln!("Failed to send ping: {:?}", e);
//                     break;
//                 }
//                 println!("Ping sent");
//             }
//         });

//         // // Handle incoming messages
//         // tokio::spawn(async move {
//         //     while let Some(message) = read.next().await {
//         //         match message {
//         //             Ok(Message::Binary(bin_msg)) => {
//         //                 let ltp_data = parse_ltp(&bin_msg).await;
//         //                 on_message(ltp_data);
//         //             }
//         //             Ok(Message::Pong(_)) => {
//         //                 println!("Received pong");
//         //             }
//         //             Ok(Message::Close(_)) => {
//         //                 println!("WebSocket closed");
//         //                 break;
//         //             }
//         //             Err(e) => {
//         //                 println!("Error receiving message: {:?}", e);
//         //             }
//         //             _ => {}
//         //         }
//         //     }
//         // });

//         let read_task = tokio::spawn({
//             let write_clone = write.clone(); // Clone `write` for later use

//             async move {
//                 while let Some(msg) = read.next().await {
//                     match msg {
//                         Ok(message) => match message {
//                             Message::Binary(bin_msg) => {
//                                 let ltp_data = parse_ltp(&bin_msg).await;
//                                 on_message(ltp_data);
//                             }
//                             Message::Text(text) => {
//                                 println!("WS MESSAGE RECEIVED : {:?}", text);
//                             }
//                             Message::Pong(payload) => {
//                                 println!("Pong Payload = {:?}", payload);
//                             }
//                             _ => {
//                                 println!("Other message type received");
//                             }
//                         },
//                         Err(e) => {
//                             eprintln!("Error receiving message: {}", e);
//                         }
//                     }
//                 }
//             }
//         });
//     });
// }

use serde::Serialize;

#[derive(Debug, Serialize_repr, Deserialize_repr, PartialEq, Eq, Clone, Copy)]
#[repr(u8)]
/// Action subscribe or not subscribe
pub enum SubscriptionAction {
    /// Unsubscribe action
    UnSubscribe = 0,
    /// Subscribe action
    Subscribe = 1,
}

#[derive(Debug, Serialize_repr, Deserialize_repr, PartialEq, Eq, Clone, Copy)]
#[repr(u8)]
/// Subscription mode Ltp or Quote
pub enum SubscriptionMode {
    /// Last traded price
    Ltp = 1,
    /// Quote data
    Quote = 2,
    /// Snap quote data
    SnapQuote = 3,
    /// Depth
    Depth = 4,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
/// Subscription request to subscribe data
pub struct SubscriptionRequest {
    #[serde(rename = "correlation_id")]
    correlation_id: String,
    action: SubscriptionAction,
    params: SubscriptionParams,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct SubscriptionParams {
    mode: SubscriptionMode,
    #[serde(rename = "tokenList")]
    token_list: Vec<TokenData>,
}

#[derive(Debug, Serialize_repr, Deserialize_repr, PartialEq, Eq, Clone, Copy)]
#[repr(u8)]
/// Exchange to subscribe
pub enum SubscriptionExchange {
    /// NSE Eq
    NSECM = 1,
    /// NSE FNO
    NSEFO = 2,
    /// BSE Eq
    BSECM = 3,
    /// BSE FNO
    BSEFO = 4,
    /// MCX FNO
    MCXFO = 5,
    /// NCX FNO
    NCXFO = 7,
    /// CDE FNO
    CDEFO = 13,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct TokenData {
    #[serde(rename = "exchangeType")]
    exchange_type: SubscriptionExchange,
    tokens: Vec<String>,
}

// Define the structure for the parsed data
#[derive(Debug, Serialize, Deserialize)]
struct ParsedData {
    subscription_mode: u8,
    ltp: f32, // Adjusted LTP to be decimal
    exchange_type: u8,
    token: String,
    sequence_number: u64,
    exchange_timestamp: u64,
    exchange_date: String,
}

// Function to read binary data and parse it
fn parse_ltp(buffer: &[u8]) -> ParsedData {
    // Helper function to read strings from bytes
    fn read_string(buffer: &[u8], start: usize, len: usize) -> String {
        let bytes = &buffer[start..start + len];
        let string = str::from_utf8(bytes)
            .unwrap_or("")
            .trim_end_matches(char::from(0));
        string.to_string()
    }

    // Read the fields from the buffer
    let subscription_mode = buffer[0];
    let exchange_type = buffer[1];
    let token = read_string(buffer, 2, 25);
    let sequence_number = LittleEndian::read_u64(&buffer[27..35]);
    let exchange_timestamp = LittleEndian::read_u64(&buffer[35..43]);
    let ltp = LittleEndian::read_i32(&buffer[43..47]) as f32 / 100.0; // Adjust LTP to decimal

    // Convert the exchange timestamp to a string date (ISO 8601 format)
    let exchange_date =
        chrono::NaiveDateTime::from_timestamp((exchange_timestamp / 1000) as i64, 0)
            .format("%Y-%m-%dT%H:%M:%S")
            .to_string();

    // Return the parsed data
    ParsedData {
        subscription_mode,
        exchange_type,
        token,
        sequence_number,
        exchange_timestamp,
        exchange_date,
        ltp,
    }
}

#[derive(Serialize, Deserialize, Debug)]
/// Build a new subscription format
pub struct SubscriptionBuilder {
    correlation_id: String,     // Correlation ID for the subscription
    action: SubscriptionAction, // Action to perform (default is subscribe with action = 1)
    mode: SubscriptionMode,     // Subscription mode (default is LTP mode = 1)
    token_list: Vec<TokenData>, // A list of exchanges and their corresponding tokens
}

impl SubscriptionBuilder {
    /// * `correlation_id` - A unique identifier for the subscription.
    pub fn new(correlation_id: &str) -> Self {
        SubscriptionBuilder {
            correlation_id: correlation_id.to_string(),
            action: SubscriptionAction::Subscribe, // Default to Subscribe (action = 1)
            mode: SubscriptionMode::Ltp,           // Default to LTP mode (mode = 1)
            token_list: vec![],                    // Initialize an empty token list
        }
    }

    /// Returns the modified `SubscriptionBuilder` instance with the updated action.
    pub fn action(mut self, action: SubscriptionAction) -> Self {
        self.action = action;
        self
    }

    /// Sets the subscription mode (e.g., LTP, etc.).
    ///
    /// # Arguments
    ///
    /// * `mode` - The mode of subscription (e.g., 1 for LTP).
    ///
    /// # Returns
    ///
    /// Returns the modified `SubscriptionBuilder` instance with the updated mode.
    pub fn mode(mut self, mode: SubscriptionMode) -> Self {
        self.mode = mode;
        self
    }

    /// Returns the modified `SubscriptionBuilder` instance with the updated token list.
    pub fn subscribe(mut self, exchange_type: SubscriptionExchange, tokens: Vec<&str>) -> Self {
        let token_list_entry = TokenData {
            exchange_type,
            tokens: tokens.into_iter().map(String::from).collect(), // Convert &str to String
        };
        self.token_list.push(token_list_entry); // Add the new token list entry
        self
    }

    /// Builds the final `SubscriptionRequest` from the current state of the builder.
    ///
    /// # Returns
    ///
    /// Returns a `SubscriptionRequest` containing the correlation ID, action, mode, and token list.
    pub fn build(self) -> SubscriptionRequest {
        SubscriptionRequest {
            correlation_id: self.correlation_id,
            action: SubscriptionAction::Subscribe,
            params: SubscriptionParams {
                mode: SubscriptionMode::Ltp,
                token_list: self.token_list,
            },
        }
    }
}

/// Angelone Websocket client with ping function
pub struct AngelOneWebSocketClient;

impl AngelOneWebSocketClient {
    /// Initialize and spawn the WebSocket client task
    pub async fn connect(
        ws_url: String,
        mut rx: mpsc::Receiver<Signal>,
        // tx: mpsc::Sender<Signal>,
        tx: Arc<tokio::sync::broadcast::Sender<Signal>>, // on_message:impl Fn(Ltp)
    ) {
        let tx_clone = tx.clone();
        // Spawn the WebSocket client task
        tokio::task::spawn(async move {
            // Connect to the WebSocket server
            // println!("Ws url = {}", ws_url);
            let (ws_stream, _) = connect_async(ws_url).await.expect("Failed to connect");
            println!("Angel One WebSocket connection established");

            // Split the WebSocket stream into a sender (sink) and receiver (stream)
            let (write, mut read) = ws_stream.split();

            let write = Arc::new(Mutex::new(write));

            // Task to handle incoming WebSocket messages
            let read_task = tokio::spawn({
                let _write_clone = write.clone(); // Clone `write` for later use

                async move {
                    while let Some(msg) = read.next().await {
                        match msg {
                            Ok(message) => match message {
                                Message::Binary(bin_msg) => {
                                    let ltp_data = parse_ltp(&bin_msg);

                                    // tx_clone
                                    //     .send(Signal::PriceFeed {
                                    //         token: ltp_data.token,
                                    //         ltp: ltp_data.ltp,
                                    //     })
                                    //     .await
                                    //     .unwrap();

                                    tx_clone
                                        .send(Signal::PriceFeed {
                                            token: ltp_data.token,
                                            ltp: ltp_data.ltp,
                                        })
                                        .unwrap();
                                }
                                Message::Text(text) => {
                                    println!("ANGELONE RECEIVED : {:?}", text);
                                }
                                Message::Pong(_payload) => {
                                    // println!("Angelone Pong Received = {:?}", payload);
                                }
                                _ => {
                                    println!("Other message type received");
                                }
                            },
                            Err(e) => {
                                eprintln!("Error receiving message: {}", e);
                            }
                        }
                    }
                }
            });

            // Task to listen for messages from the main thread and send them to the WebSocket
            let write_clone = write.clone();
            tokio::spawn(async move {
                while let Some(message) = rx.recv().await {
                    match message {
                        Signal::Subscribe(sub_req) => {
                            println!("Received .... {:?}", sub_req);
                            // println!("Serialized {:?}", serde_json::to_string(&sub_req));
                            let mut write_locked = write_clone.lock().await;
                            let msg_to_send =
                                Message::Text(serde_json::to_string(&sub_req).unwrap());
                            // println!("SENDING MESSAGES : {:?}", msg_to_send);
                            if let Err(e) = write_locked.send(msg_to_send).await {
                                eprintln!("Failed to send message: {}", e);
                            }
                        }
                        _ => {}
                    }
                }
            });

            // Task to send ping messages periodically every 30 seconds
            tokio::spawn({
                let write_clone = write.clone();

                async move {
                    loop {
                        sleep(Duration::from_secs(30)).await;
                        let mut write_locked = write_clone.lock().await;
                        if let Err(e) = write_locked.send(Message::Ping(vec![])).await {
                            eprintln!("Error sending ping: {:?}", e);
                            break;
                        }
                        // println!("Angel One Sent ping");
                    }
                }
            });

            // Wait for the read task to finish
            if let Err(e) = read_task.await {
                eprintln!("\n\nRead task failed: {}", e);
            }

            println!("\n\nWebSocket connection closed.");
        });
    }
}
