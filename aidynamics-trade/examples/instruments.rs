use aidynamics_trade::SmartConnect;

#[tokio::main]
async fn main() {
    let instruments = SmartConnect::instruments().await.unwrap();

    println!("{:?}", instruments);
}
