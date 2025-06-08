

// use redis::{AsyncCommands, RedisResult, Value};
// use tokio::time::{sleep, Duration};

// const STREAM_NAME: &str = "mystream";
// const CONSUMER_GROUP: &str = "mygroup";
// const CONSUMER_NAME: &str = "consumer1";

// #[tokio::main]
// async fn main() -> RedisResult<()> {
//     // Create a Redis connection manager
//     let client = redis::Client::open("redis://127.0.0.1/")?;
//     let mut con = client.get_async_connection().await?;

//     // Ensure the consumer group exists
//     let _: RedisResult<()> = create_consumer_group(&mut con).await;

//     // Consume messages from the stream
//     loop {
//         match consume_messages(&mut con).await {
//             Ok(_) => {}
//             Err(err) => eprintln!("Error consuming messages: {:?}", err),
//         }

//         // Sleep before the next iteration (polling interval)
//         sleep(Duration::from_secs(1)).await;
//     }
// }

// /// Create a consumer group for the stream if it doesn't exist
// async fn create_consumer_group(con: &mut redis::aio::Connection) -> RedisResult<()> {
//     let result: RedisResult<()> = con
//         .xgroup_create_mkstream(STREAM_NAME, CONSUMER_GROUP, "$")
//         .await;

//     if let Err(err) = result {
//         if err.to_string().contains("BUSYGROUP") {
//             println!("Consumer group '{}' already exists.", CONSUMER_GROUP);
//             return Ok(());
//         } else {
//             return Err(err); // No need to clone since `err` is already owned
//         }
//     }

//     println!("Consumer group '{}' created.", CONSUMER_GROUP);
//     Ok(())
// }

// /// Consume messages from the Redis stream
// async fn consume_messages(con: &mut redis::aio::Connection) -> RedisResult<()> {
//     // Read messages using XREADGROUP
//     let messages: RedisResult<Value> = con
//         .xread_options(
//             &[STREAM_NAME], // Stream names as an array
//             redis::streams::StreamReadOptions::default()
//                 .group(CONSUMER_GROUP, CONSUMER_NAME) // Use consumer group
//                 .count(10) // Max number of messages to fetch
//                 .block(1000), // Block for 1 second if no message
//             &[">"],         // IDs for each stream
//         )
//         .await;

//     match messages {
//         Ok(Value::Bulk(streams)) => {
//             for stream in streams {
//                 if let Value::Bulk(entries) = stream {
//                     for entry in entries {
//                         process_message(entry).await?;
//                     }
//                 }
//             }
//         }
//         Ok(_) => {
//             // No messages to process
//         }
//         Err(err) => {
//             eprintln!("Error reading from stream: {:?}", err);
//         }
//     }

//     Ok(())
// }

// /// Process a single message
// async fn process_message(message: Value) -> RedisResult<()> {
//     if let Value::Bulk(entry) = message {
//         // Extract ID and fields
//         if entry.len() >= 2 {
//             if let (Value::Data(id), Value::Bulk(fields)) = (&entry[0], &entry[1]) {
//                 let id_str = String::from_utf8_lossy(id);

//                 println!("Processing message with ID: {}", id_str);
//                 println!("Fields: {:?}", fields);

//                 // Acknowledge the message
//                 // Example: xack mystream mygroup <id>
//                 // Redis acknowledgment can be added here if needed
//             }
//         }
//     }

//     Ok(())
// }





// use redis::{streams::StreamReadOptions, AsyncCommands, RedisResult};
// use tokio::time::{sleep, Duration};
// use tokio;

// #[tokio::main]
// async fn main() -> RedisResult<()> {
//     let client = redis::Client::open("redis://127.0.0.1/")?;
//     let mut con = client.get_async_connection().await?;

//     // Create a consumer group
//     let _: () = con.xgroup_create_mkstream("mystream", "mygroup", "$").await?;

//     // Consume messages from the stream
//     loop {
//         let opts = StreamReadOptions::default()
//     .count(10);
//         let msgs: Vec<redis::streams::StreamReadReply> = con.xread_options(
//             &["mystream"],
//             &["0"],
//             &opts.group("mygroup", "consumer1").count(10).block(0)
//         ).await?;

//         for msg in msgs {
//             for stream in msg.keys {
//                 for message in stream.ids {
//                     println!("Received message: {:?}", message.map);
//                     // Acknowledge the message
//                     let _: () = con.xack("mystream", "mygroup", &[message.id]).await?;
//                 }
//             }
//         }

//         sleep(Duration::from_secs(1)).await;
//     }
// }




use redis::{aio::MultiplexedConnection, AsyncCommands, Client, RedisResult};
use redis::streams::{StreamReadOptions, StreamReadReply};
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> RedisResult<()> {
    // Connect to Redis
    let redis_url = "redis://127.0.0.1/"; // Change as needed
    let client = Client::open(redis_url)?;
    let mut con: MultiplexedConnection = client.get_multiplexed_async_connection().await?;

    let stream_name = "aid_stream";
    let group_name = "aid_group";
    let consumer_name = "aid_consumer";

    // Attempt to create the consumer group
    // let result = con
    //     .xgroup_create_mkstream(stream_name, group_name, "$")
    //     .await;

    // if let Err(err) = result {
    //     if err.to_string().contains("BUSYGROUP") {
    //         println!("Consumer group '{}' already exists.", group_name);
    //     } else {
    //         panic!("Error creating consumer group: {}", err);
    //     }
    // }

    // Consume messages from the stream
    loop {
        let opts = StreamReadOptions::default().count(10).block(1000);
        let reply: RedisResult<StreamReadReply> = con
            .xread_options(
                &[stream_name],
                &["0"],
                &opts.group(group_name, consumer_name),
            )
            .await;

        match reply {
            Ok(stream_reply) => {
                for stream in stream_reply.keys {
                    for message in stream.ids {
                        println!("Received message: {:?}", message.map);

                        // Acknowledge the message
                        let _: () = con
                            .xack(stream_name, group_name, &[message.id])
                            .await
                            .unwrap();
                    }
                }
            }
            Err(err) => {
                println!("Error reading messages: {}", err);
            }
        }

        sleep(Duration::from_secs(1)).await;
    }
}
