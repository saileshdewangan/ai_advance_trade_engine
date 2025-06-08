// use std::str;
// use byteorder::{ByteOrder, LittleEndian}; use futures::stream::SplitSink;
// // You can use the `byteorder` crate for handling endian conversions
// use tokio::{net::TcpStream, time::{interval, Duration}};
// use futures_util::{StreamExt, SinkExt};
// use tokio_tungstenite::{connect_async, tungstenite::protocol::Message, MaybeTlsStream, WebSocketStream};
// use serde::{Deserialize, Serialize};
// use std::time::Instant;

// #[derive(Serialize, Deserialize, Debug)]
// struct SubscriptionRequest {
//     #[serde(rename = "correlationID")]
//     correlation_id: String,
//     action: i32,
//     params: SubscriptionParams,
// }

// #[derive(Serialize, Deserialize, Debug)]
// struct SubscriptionParams {
//     mode: i32,
//     #[serde(rename = "tokenList")]
//     token_list: Vec<TokenData>,
// }

// #[derive(Serialize, Deserialize, Debug)]
// struct TokenData {
//     #[serde(rename = "exchangeType")]
//     exchange_type: i32,
//     tokens: Vec<String>,
// }

// // Define the structure for the parsed data
// #[derive(Debug,Serialize,Deserialize)]
// struct ParsedData {
//     subscription_mode: u8,
//     ltp: f32, // Adjusted LTP to be decimal
//     exchange_type: u8,
//     token: String,
//     sequence_number: u64,
//     exchange_timestamp: u64,
//     exchange_date: String,
// }

// // Function to read binary data and parse it
// fn parse_ltp(buffer: &[u8]) -> ParsedData {
//     // Helper function to read strings from bytes
//     fn read_string(buffer: &[u8], start: usize, len: usize) -> String {
//         let bytes = &buffer[start..start + len];
//         let string = str::from_utf8(bytes).unwrap_or("").trim_end_matches(char::from(0));
//         string.to_string()
//     }

//     // Read the fields from the buffer
//     let subscription_mode = buffer[0];
//     let exchange_type = buffer[1];
//     let token = read_string(buffer, 2, 25);
//     let sequence_number = LittleEndian::read_u64(&buffer[27..35]);
//     let exchange_timestamp = LittleEndian::read_u64(&buffer[35..43]);
//     let ltp = LittleEndian::read_i32(&buffer[43..47]) as f32 / 100.0; // Adjust LTP to decimal

//     // Convert the exchange timestamp to a string date (ISO 8601 format)
//     let exchange_date = chrono::NaiveDateTime::from_timestamp((exchange_timestamp / 1000) as i64, 0)
//         .format("%Y-%m-%dT%H:%M:%S")
//         .to_string();

//     // Return the parsed data
//     ParsedData {
//         subscription_mode,
//         exchange_type,
//         token,
//         sequence_number,
//         exchange_timestamp,
//         exchange_date,
//         ltp,
//     }
// }

// async fn send_heartbeat(ws_sink: &mut SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>) {
//     // Construct your heartbeat message
//     let heartbeat_message = Message::Ping(vec![]); // Example heartbeat message
//     if let Err(e) = ws_sink.send(heartbeat_message).await {
//         eprintln!("Error sending heartbeat: {:?}", e);
//     }
//     else {
//         println!("\n\n\n\n Ping sent");
//     }
// }

// #[tokio::main]
// async fn main() {
//     let (ws_stream, _) = connect_async("wss://smartapisocket.angelone.in/smart-stream?clientCode=S55591176&feedToken=eyJhbGciOiJIUzUxMiJ9.eyJ1c2VybmFtZSI6IlM1NTU5MTE3NiIsImlhdCI6MTcyOTcyNzkxMCwiZXhwIjoxNzI5ODE0MzEwfQ.QELdVmgDqNStmIWMsMWYXtNjnXuynaJUyaccK_C3hvuXzVZd7GRm_sF80TUkBrOBL1pLfLy9ttvex_Fz_AkUJA&apiKey=SDr8GANb")
//         .await.expect("Failed to connect");

//     println!("WebSocket connection established");

//     let (mut ws_sink, mut ws_stream) = ws_stream.split();

//     // Send a subscription request on WebSocket open
//     let subscription_request = SubscriptionRequest {
//         correlation_id: "abcde12345".to_string(),
//         action: 1, // Subscribe
//         params: SubscriptionParams {
//             mode: 1, // LTP mode
//             token_list: vec![
//                 TokenData { exchange_type: 1, tokens: vec!["26009".into(), "26000".into(),"26037".into()] },
//                 TokenData { exchange_type: 2, tokens: vec!["43125".into()] },
//                 TokenData { exchange_type: 3, tokens: vec!["19000".into()] },
//                 TokenData { exchange_type: 5, tokens: vec!["429116".into()] },
//             ],
//         },
//     };

//     // Serialize subscription request to JSON and send
//     let sub_msg = serde_json::to_string(&subscription_request).unwrap();
//     ws_sink.send(Message::Text(sub_msg)).await.unwrap();

//     // Start a heartbeat loop to send a ping every 30 seconds
//     let mut heartbeat_interval = interval(Duration::from_secs(30));
//     tokio::spawn(async move {
//         loop {
//             heartbeat_interval.tick().await;
//             send_heartbeat(&mut ws_sink).await;
//         }
//     });

//     // Handle incoming WebSocket messages (like LTP updates)
//     while let Some(message) = ws_stream.next().await {
//         match message {
//             Ok(Message::Text(text)) => {
//                 println!("\n\nReceived text message: {}", text);
//                 // Handle JSON text messages here if needed
//             }
//             Ok(Message::Binary(bin)) => {
//                 // Handle binary data (like LTP feed)
//                 // let parsed_data = parse_binary_message(&bin);
//                 let parsed_data = parse_ltp(&bin);
//                 println!("\n\nParsed Data: {:?}", parsed_data.ltp);
//             }
//             Ok(Message::Pong(_)) => {
//                 println!("\n\nReceived Pong response");
//             }
//             Err(e) => {
//                 eprintln!("\n\nWebSocket error: {}", e);
//             }
//             _ => {}
//         }
//     }
// }




// use byteorder::{ByteOrder, LittleEndian};
// use futures::stream::SplitSink;
// use serde_repr::{Serialize_repr,Deserialize_repr};
// use std::str;
// // You can use the `byteorder` crate for handling endian conversions
// use futures_util::{SinkExt, StreamExt};
// use serde::{Deserialize, Serialize};
// use std::time::Instant;
// use tokio::{
//     net::TcpStream,
//     time::{interval, Duration},
// };
// use tokio_tungstenite::{
//     connect_async, tungstenite::protocol::Message, MaybeTlsStream, WebSocketStream,
// };


// #[derive(Debug, Serialize_repr, Deserialize_repr, PartialEq, Eq, Clone, Copy)]
// #[repr(u8)]
// /// Action subscribe or not subscribe
// pub enum SubscriptionAction {
//     /// Unsubscribe action
//     UnSubscribe = 0,
//     /// Subscribe action
//     Subscribe = 1,
// }

// #[derive(Debug, Serialize_repr, Deserialize_repr, PartialEq, Eq, Clone, Copy)]
// #[repr(u8)]
// /// Subscription mode Ltp or Quote
// pub enum SubscriptionMode {
//     /// Last traded price
//     Ltp = 1,
//     /// Quote data
//     Quote = 2,
//     /// Snap quote data
//     SnapQuote = 3,
//     /// Depth
//     Depth = 4,
// }

// #[derive(Serialize, Deserialize, Debug)]
// struct SubscriptionRequest {
//     #[serde(rename = "correlation_id")]
//     correlation_id: String,
//     action: SubscriptionAction,
//     params: SubscriptionParams,
// }

// #[derive(Serialize, Deserialize, Debug)]
// struct SubscriptionParams {
//     mode: SubscriptionMode,
//     #[serde(rename = "tokenList")]
//     token_list: Vec<TokenData>,
// }

// #[derive(Debug, Serialize_repr, Deserialize_repr, PartialEq, Eq, Clone, Copy)]
// #[repr(u8)]
// /// Exchange to subscribe
// pub enum SubscriptionExchange {
//     /// NSE Eq
//     NSECM = 1,
//     /// NSE FNO
//     NSEFO = 2,
//     /// BSE Eq
//     BSECM = 3,
//     /// BSE FNO
//     BSEFO = 4,
//     /// MCX FNO
//     MCXFO = 5,
//     /// NCX FNO
//     NCXFO = 7,
//     /// CDE FNO
//     CDEFO = 13,
// }

// #[derive(Serialize, Deserialize, Debug)]
// struct TokenData {
//     #[serde(rename = "exchangeType")]
//     exchange_type: SubscriptionExchange,
//     tokens: Vec<String>,
// }

// // Define the structure for the parsed data
// #[derive(Debug, Serialize, Deserialize)]
// struct ParsedData {
//     subscription_mode: u8,
//     ltp: f32, // Adjusted LTP to be decimal
//     exchange_type: u8,
//     token: String,
//     sequence_number: u64,
//     exchange_timestamp: u64,
//     exchange_date: String,
// }

// // Function to read binary data and parse it
// fn parse_ltp(buffer: &[u8]) -> ParsedData {
//     // Helper function to read strings from bytes
//     fn read_string(buffer: &[u8], start: usize, len: usize) -> String {
//         let bytes = &buffer[start..start + len];
//         let string = str::from_utf8(bytes)
//             .unwrap_or("")
//             .trim_end_matches(char::from(0));
//         string.to_string()
//     }

//     // Read the fields from the buffer
//     let subscription_mode = buffer[0];
//     let exchange_type = buffer[1];
//     let token = read_string(buffer, 2, 25);
//     let sequence_number = LittleEndian::read_u64(&buffer[27..35]);
//     let exchange_timestamp = LittleEndian::read_u64(&buffer[35..43]);
//     let ltp = LittleEndian::read_i32(&buffer[43..47]) as f32 / 100.0; // Adjust LTP to decimal

//     // Convert the exchange timestamp to a string date (ISO 8601 format)
//     let exchange_date =
//         chrono::NaiveDateTime::from_timestamp((exchange_timestamp / 1000) as i64, 0)
//             .format("%Y-%m-%dT%H:%M:%S")
//             .to_string();

//     // Return the parsed data
//     ParsedData {
//         subscription_mode,
//         exchange_type,
//         token,
//         sequence_number,
//         exchange_timestamp,
//         exchange_date,
//         ltp,
//     }
// }



use byteorder::{ByteOrder, LittleEndian};
use futures::stream::SplitSink;
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::{str, sync::Arc};
// You can use the `byteorder` crate for handling endian conversions
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::time::Instant;
use tokio::{
    net::TcpStream,
    sync::Mutex,
    time::{interval, Duration},
};
use tokio_tungstenite::{
    connect_async, tungstenite::protocol::Message, MaybeTlsStream, WebSocketStream,
};

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

#[derive(Serialize, Deserialize, Debug)]
struct SubscriptionRequest {
    #[serde(rename = "correlationID")]
    correlation_id: String,
    action: SubscriptionAction,
    params: SubscriptionParams,
}

#[derive(Serialize, Deserialize, Debug)]
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

#[derive(Serialize, Deserialize, Debug)]
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

async fn send_heartbeat(
    ws_sink: Arc<Mutex<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>>,
) {
    let heartbeat_message = Message::Ping(vec![]);
    let mut lock = ws_sink.lock().await; // Lock is acquired here
    if let Err(e) = lock.send(heartbeat_message).await {
        eprintln!("Error sending heartbeat: {:?}", e);
    } else {
        println!("\n\n\n\n Ping sent");
    }
}

#[tokio::main]
async fn main() {
    let (ws_stream, _) = connect_async("wss://smartapisocket.angelone.in/smart-stream?clientCode=S55591176&feedToken=eyJhbGciOiJIUzUxMiJ9.eyJ1c2VybmFtZSI6IlM1NTU5MTE3NiIsImlhdCI6MTcyOTcyNzkxMCwiZXhwIjoxNzI5ODE0MzEwfQ.QELdVmgDqNStmIWMsMWYXtNjnXuynaJUyaccK_C3hvuXzVZd7GRm_sF80TUkBrOBL1pLfLy9ttvex_Fz_AkUJA&apiKey=SDr8GANb")
        .await.expect("Failed to connect");

    println!("WebSocket connection established");

    let (mut ws_sink, mut ws_stream) = ws_stream.split();

    let ws_sink = Arc::new(Mutex::new(ws_sink));

    // Send a subscription request on WebSocket open
    let subscription_request = SubscriptionRequest {
        correlation_id: "abcde12345".to_string(),
        action: SubscriptionAction::Subscribe, // Subscribe
        params: SubscriptionParams {
            mode: SubscriptionMode::Ltp, // LTP mode
            token_list: vec![
                TokenData {
                    exchange_type: SubscriptionExchange::NSECM,
                    tokens: vec!["26009".into(), "26000".into(), "26037".into()],
                },
                TokenData {
                    exchange_type: SubscriptionExchange::NSEFO,
                    tokens: vec!["43125".into()],
                },
                TokenData {
                    exchange_type: SubscriptionExchange::BSECM,
                    tokens: vec!["19000".into()],
                },
                TokenData {
                    exchange_type: SubscriptionExchange::MCXFO,
                    tokens: vec!["429116".into()],
                },
            ],
        },
    };

    // Serialize subscription request to JSON and send
    let sub_msg = serde_json::to_string(&subscription_request).unwrap();
    {
        let mut write = ws_sink.lock().await;
        println!("SENDING MSG {:?}",sub_msg);
        write.send(Message::Text(sub_msg)).await.unwrap();
    }
    // ws_sink.send(Message::Text(sub_msg)).await.unwrap();

    let heartbeat_ws_sink = ws_sink.clone();
    // Start a heartbeat loop to send a ping every 30 seconds
    tokio::spawn(async move {
        let mut heartbeat_interval = interval(Duration::from_secs(30));
        loop {
            heartbeat_interval.tick().await;
            send_heartbeat(heartbeat_ws_sink.clone()).await;
        }
    });

    // Handle incoming WebSocket messages (like LTP updates)
    while let Some(message) = ws_stream.next().await {
        match message {
            Ok(Message::Text(text)) => {
                println!("\n\nReceived text message: {}", text);
                // Handle JSON text messages here if needed
            }
            Ok(Message::Binary(bin)) => {
                // Handle binary data (like LTP feed)
                // let parsed_data = parse_binary_message(&bin);
                let parsed_data = parse_ltp(&bin);
                println!("\n\nParsed Data: {:?}", parsed_data.ltp);
            }
            Ok(Message::Pong(_)) => {
                println!("\n\nReceived Pong response");
            }
            Err(e) => {
                eprintln!("\n\nWebSocket error: {}", e);
            }
            _ => {}
        }
    }
}
