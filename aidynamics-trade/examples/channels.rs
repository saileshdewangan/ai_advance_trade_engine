// use std::sync::mpsc::Receiver;

use std::{thread::sleep, time::Duration};

use rand::random;
use tokio::sync::mpsc::Receiver;

// use futures::channel::mpsc::Receiver;

#[derive(Debug)]
struct Msg{
    state:i32,
    computation:i32
}



#[tokio::main]
async fn main() {
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Msg>(100);

    async fn receive_messages(rx: &mut Receiver<Msg>) {
        while let Some(msg) = rx.recv().await {
            // Process the message
            println!("Message is {:?}",msg);
        }
    }

    // Spawn the task, passing a mutable reference
    tokio::spawn(async move {
        receive_messages(&mut rx).await;
        sleep(Duration::from_secs(5));
    });

    // You can use `tx` to send messages here

    for _ in 0..10{
        tx.send(Msg{state:random(),computation:random()}).await.unwrap();
    }
}