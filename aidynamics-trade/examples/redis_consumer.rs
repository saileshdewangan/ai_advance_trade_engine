use redis::{Client, Commands, Connection};
use redis_streams::consumer::{Consumer, ConsumerOpts, Message};
use std::thread;
use std::time::Duration;

fn main() -> redis::RedisResult<()> {
    let client = Client::open("redis://127.0.0.1/")?;

    // Spawn multiple threads for consumers
    let num_consumers = 3; // Number of consumers
    let stream_name = "mystream";
    let group_name = "mygroup";

    for i in 0..num_consumers {
        let client_clone = client.clone(); // Clone the client for each thread
        let consumer_name = format!("consumer_{}", i); // Unique consumer name

        thread::spawn(move || -> redis::RedisResult<()> {
            let mut con = client_clone.get_connection()?;

            // Create consumer group if it doesn't exist (only needed once)
            let _ : () = con.xgroup_create_mkstream(stream_name, group_name, "$")
                .unwrap_or_else(|err| {
                    if err.kind() != redis::ErrorKind::RedisError {
                        panic!("Redis error: {}", err);
                    }
                    // Ignore "Group already exists" error
                });

            let opts = ConsumerOpts::default().group(group_name, &consumer_name).count(1).block(5000); // Block for 5 seconds
            let mut consumer = Consumer::init(&mut con, stream_name, |id: &str, message: &Message| {
                println!("{}: Received message: ID={}, Data={:?}", consumer_name, id, message.map);
                Ok(())
            }, opts).unwrap();

            loop {
                match consumer.consume() {
                    Ok(_) => {}, // Messages processed
                    Err(e) => println!("{}: Error consuming: {}", consumer_name, e),
                }
                 // Optional: Add a small delay between iterations
                thread::sleep(Duration::from_millis(100));
            }

        });
    }

    // Keep the main thread alive (you might have other tasks here)
    loop {
        thread::sleep(Duration::from_secs(1));
    }
}