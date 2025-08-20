use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::io::stdin;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::time::sleep;
use tokio_tungstenite::tungstenite::{Error, Message};
use url::Url;
use std::io;

// Define the data structures for our messages
#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
enum Payload {
    EventPayload {
        event: String,
        data: serde_json::Value,
    },
    MessagePayload {
        channel: Option<String>,
        sender_id: String,
        content: String,
        timestamp: String,
    },
    StatusPayload {
        event: String,
        data: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    // The main connection loop. This loop will run indefinitely.
    loop {
        // Attempt to connect to the WebSocket server
        let url = "ws://127.0.0.1:5001".to_string();
        println!("Attempting to connect to {}", url);

        let connect_result = tokio_tungstenite::connect_async(url).await;

        match connect_result {
            Ok((mut ws_stream, _)) => {
                println!("✅ Successfully connected to WebSocket server!");
                println!("Type commands to interact with the server. e.g., 'register your_id'");

                // Create channels to communicate between tasks
                let (tx, mut rx) = tokio::sync::mpsc::channel(32);
                let ws_tx = tx.clone();

                // Spawn a task to handle incoming WebSocket messages
                let ws_handle = tokio::spawn(async move {
                    while let Some(msg) = ws_stream.next().await {
                        match msg {
                            Ok(Message::Binary(buf)) => {
                                let text = String::from_utf8_lossy(&buf);

                                if text.starts_with(r#"{"event":"status""#) {
                                    if let Ok(status_msg) = serde_json::from_str::<Payload>(&text) {
                                        println!("[STATUS] {:?}", status_msg);
                                    }
                                } else if let Ok(message_msg) =
                                    serde_json::from_str::<Payload>(&text)
                                {
                                    if let Payload::MessagePayload {
                                        channel,
                                        sender_id,
                                        content,
                                        ..
                                    } = message_msg
                                    {
                                        let channel_prefix = channel
                                            .map(|c| format!("[{}] ", c))
                                            .unwrap_or_default();
                                        println!("{}{} : {}", channel_prefix, sender_id, content);
                                    }
                                } else {
                                    println!("Received unknown binary message: {}", text);
                                }
                            }
                            Ok(Message::Text(text)) => {
                                println!("[TEXT] {}", text);
                            }
                            Ok(Message::Close(close_frame)) => {
                                println!("Disconnected from server: {:?}", close_frame);
                                break;
                            }
                            Err(e) => {
                                eprintln!("WebSocket Error: {}", e);
                                break;
                            }
                            _ => {}
                        }
                    }
                });

                // Main loop for user input
                let mut reader = BufReader::new(stdin());
                let mut line = String::new();

                loop {
                    tokio::select! {
                        // Read user input
                        bytes_read = reader.read_line(&mut line) => {
                            let bytes_read = bytes_read.expect("Failed to read from stdin");
                            if bytes_read == 0 {
                                // EOF, close the connection
                                println!("Closing connection due to end of input.");
                                break;
                            }

                            let input = line.trim();
                            let parts: Vec<&str> = input.splitn(2, ' ').collect();
                            if parts.len() < 2 {
                                println!("Invalid command. Use format: 'command data'");
                                line.clear();
                                continue;
                            }

                            let command = parts[0];
                            let data = parts[1];

                            let payload_json = match command {
                                "register" => {
                                    serde_json::json!({ "event": "register", "data": data })
                                },
                                "subscribe" => {
                                    serde_json::json!({ "event": "subscribe", "data": data })
                                },
                                "unsubscribe" => {
                                    serde_json::json!({ "event": "unsubscribe", "data": data })
                                },
                                "message" => {
                                    let msg_parts: Vec<&str> = data.splitn(2, ' ').collect();
                                    if msg_parts.len() < 2 {
                                        println!("Invalid message format. Use: 'message channel content'");
                                        line.clear();
                                        continue;
                                    }
                                    let channel = msg_parts[0];
                                    let content = msg_parts[1];
                                    serde_json::json!({ "event": "message", "data": { "channel": channel, "content": content } })
                                },
                                "private-message" => {
                                    let msg_parts: Vec<&str> = data.splitn(2, ' ').collect();
                                    if msg_parts.len() < 2 {
                                        println!("Invalid private message format. Use: 'private-message recipient_id content'");
                                        line.clear();
                                        continue;
                                    }
                                    let recipient_id = msg_parts[0];
                                    let content = msg_parts[1];
                                    serde_json::json!({ "event": "private-message", "data": { "recipientId": recipient_id, "content": content } })
                                },
                                _ => {
                                    println!("Unknown command: {}", command);
                                    line.clear();
                                    continue;
                                }
                            };

                            // Send the payload to the WebSocket task
                            if let Err(e) = ws_tx.send(payload_json).await {
                                eprintln!("Failed to send message to WebSocket task: {}", e);
                            }
                            line.clear();
                        }
                        // Receive messages from the channel and send to the WebSocket stream
                        Some(payload) = rx.recv() => {
                            let binary_msg = Message::binary(serde_json::to_vec(&payload).expect("Failed to serialize payload"));
                            if let Err(e) = ws_stream.send(binary_msg).await {
                                eprintln!("Failed to send message to WebSocket: {}", e);
                                // A send error indicates a broken connection, so we break to reconnect
                                break;
                            }
                        }
                    }
                }

                // Wait for the WebSocket handler to finish before looping
                if let Err(e) = ws_handle.await {
                    eprintln!("Error in WebSocket handler task: {}", e);
                }
            }
            Err(e) => {
                eprintln!("Failed to connect: {}", e);
                // Wait before trying to reconnect
                sleep(Duration::from_secs(5)).await;
            }
        }
    }
}
