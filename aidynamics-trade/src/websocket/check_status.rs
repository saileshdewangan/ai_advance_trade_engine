use reqwest::Client;
use serde_json::json;
use tokio::time::{sleep, Duration};

/// Function to check server status using HTTP POST request.
pub async fn check_server_status(status_url: &str, interval: Duration) -> bool {
    let client = Client::new();
    // loop {
    let response = client
        .post(status_url)
        .json(&json!({ "request": "status" }))
        .send()
        .await;

    match response {
        Ok(resp) => {
            if resp.status().is_success() {
                println!("Server is up...");
                return true;
            } else {
                println!("Server is not ready yet. Retrying...");
                return false;
            }
        }
        Err(e) => {
            println!("Error checking server status: {:?}", e);
            sleep(interval).await;
            return false;
        }
    }
    // }
}
