#![allow(unused_variables)]
use redis::Commands;
use serde::Serialize;
// use serde_json::json;

// Define a struct for your JSON data
#[derive(Serialize)]
struct Order {
    order_id: String,
    customer: String,
    items: Vec<Item>,
    status: String,
    timestamp: String,
}

#[derive(Serialize)]
struct Item {
    product: String,
    quantity: u32,
    price: f64,
}

fn main() -> redis::RedisResult<()> {
    // Connect to the Redis server
    let client = redis::Client::open("redis://127.0.0.1/")?;
    let mut con = client.get_connection()?;

    // Prepare the data to send
    let order = Order {
        order_id: "12345".to_string(),
        customer: "John Doe".to_string(),
        items: vec![
            Item {
                product: "Laptop".to_string(),
                quantity: 1,
                price: 1200.0,
            },
            Item {
                product: "Mouse".to_string(),
                quantity: 2,
                price: 50.0,
            },
        ],
        status: "pending".to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(), // Use current timestamp
    };

    // Serialize the data to JSON
    let json_data = serde_json::to_string(&order).expect("Failed to serialize JSON");

    // Add the JSON data to the Redis stream
    let stream_name = "orders_stream";
    let result: String = con.xadd(stream_name, "*", &[("data", json_data)])?;

    println!("Message added to Redis stream with ID: {}", result);

    Ok(())
}