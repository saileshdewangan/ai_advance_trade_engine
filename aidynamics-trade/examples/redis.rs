// use std::time::Duration;

// use redis::{Commands, ControlFlow, PubSubCommands};
// use tokio::time::sleep;

// #[tokio::main]
// async fn main() {
//     let redis_host_name = "aidynamicsplatform.in";
//     let redis_password = "l42OUsgye3YIJ0+sEqXSVdrlGyfCNuGmL8V8lUzY18Q=";
//     let redis_conn_url = format!("redis://:{}@{}", redis_password, redis_host_name);

//     let redis_host_clone = redis_host_name.clone();
//     let redis_password_clone = redis_password.clone();
//     let redis_conn_url_clone = redis_conn_url.clone();

//     // let redis_conn_url = format!("{}@{}", redis_password, redis_host_name);
//     //println!("{}", redis_conn_url);

//     let client = redis::Client::open(redis_conn_url.as_str()).unwrap();
//     // .expect("Invalid connection URL")
//     // .get_async_connection()
//     // .await
//     // .unwrap();

//     // let mut con = connect();
//     let channel = String::from("broadcast");
//     let message = String::from("Hello from Rust!");

//     let channel_clone = channel.clone();

//     let mut con = client.get_connection().unwrap();
//     println!("Connected to Redis!");
//     let _ :usize = con.publish(channel, message).unwrap();

//     con.req_command("PING");

//     // let subscribers: usize = con.publish(channel.clone(), message.clone()).await.unwrap();

//     tokio::spawn(async move {
//         let client = redis::Client::open(redis_conn_url_clone.as_str()).unwrap();
//         let mut pubsub_con = client.get_connection().unwrap();
//         let _ : () = pubsub_con
//             .subscribe(&[channel_clone], |msg| {
//                 let received: String = msg.get_payload().unwrap();
//                 println!("Message is -> {:?}", received);
//                 return ControlFlow::Continue;
//             })
//             .unwrap();
//         println!("Subscribed to broadcast!");
//     });

//     loop{
//         let _ = sleep(Duration::from_secs(1));
//     }
// }

use std::time::Duration;

use redis::{AsyncCommands, Commands, ConnectionLike, ControlFlow, PubSubCommands};
use tokio::time::sleep;
use futures_util::StreamExt as _;

#[tokio::main]
async fn main() {
    let redis_host_name = "aidynamicsplatform.in";
    let redis_password = "l42OUsgye3YIJ0+sEqXSVdrlGyfCNuGmL8V8lUzY18Q=";
    let redis_conn_url = format!("redis://:{}@{}", redis_password, redis_host_name);

    let redis_host_clone = redis_host_name.clone();
    let redis_password_clone = redis_password.clone();
    let redis_conn_url_clone = redis_conn_url.clone();

    let client = redis::Client::open(redis_conn_url.as_str()).unwrap();
    // .expect("Invalid connection URL")
    // .get_async_connection()
    // .await
    // .unwrap();

    // let mut con = connect();
    let channel = String::from("broadcast");
    let message = String::from("Hello from Rust!");

    let channel_clone = channel.clone();

    let mut con = client.get_multiplexed_async_connection().await.unwrap();
    
    let _:() = redis::cmd("PING").query_async(&mut con).await.unwrap();

    con.set_response_timeout(Duration::from_secs(18000));
    println!("Connected to Redis!");
    let _: usize = con.publish(channel, message).await.unwrap();

    let mut pubsub = client.get_async_pubsub().await.unwrap();

    pubsub.subscribe(channel_clone).await.unwrap();

    // Spawn a separate task to listen for messages
    tokio::spawn(async move {
        let mut message_stream = pubsub.on_message();
        loop {
            match message_stream.next().await {
                Some(msg) => {
                    let payload: String = msg.get_payload().unwrap();
                    println!("Received message: {}", payload);
                }
                None => {
                    println!("No message received....");
                }
            }
            // let Some(msg) = message_stream.next().await {
            //     let payload: String = msg.get_payload().unwrap();
            //     println!("Received message: {}", payload);
        }
    });

    loop {
        let _ = sleep(Duration::from_secs(1));
    }
}
