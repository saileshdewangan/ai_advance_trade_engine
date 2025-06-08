use std::sync::Arc;

// use aidynamics_trade_utils::Error;
// use tokio::sync::Mutex;
use reqwest::{Client, Response};
use serde_json::Value;

use crate::{
    order::{IndividualOrderStatus, PlaceOrderReq, PlaceOrderRes},
    server_utils::{self, ServerHttp},
    types::TransactionType,
    user::Profile,
    Result, SmartConnect,
};

const INDICES_TOKENS: [&'static str; 3] = ["26009", "26000", "26037"];

/// Enum trade status
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum TradeStatus {
    /// Variant showing running status
    #[serde(rename = "open")]
    Open,
    /// Variant showing stopped status
    #[serde(rename = "closed")]
    Closed,
}

/// Enum to strategy
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum Strategy {
    /// EMA-5
    #[serde(rename = "5_ema")]
    Ema5,
    /// Stochastic RSI
    #[serde(rename = "stochastic_rsi")]
    StochasticRsi,
    /// Stochastic + RSI + MACD
    #[serde(rename = "rsi_macd_stochastic")]
    RsiMacdStochastic,
}

/// Enum to segment
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum Segment {
    /// EQ
    #[serde(rename = "eq")]
    EQ,
    /// Option
    #[serde(rename = "option")]
    OPTION,
    /// Future
    #[serde(rename = "future")]
    FUTURE,
}

/// Trade struct for trade details

#[derive(Debug, Serialize, Deserialize)]
pub struct Trade {
    /// Trade id
    #[serde(rename = "tradeid")]
    pub trade_id: u32,
    /// status response
    pub status: String,
    /// Comment
    pub comment: String,
}

/// Configuration for creating a new `TradeHandler`.
///
/// This struct holds all the necessary parameters to initialize a `TradeHandler`
/// including API credentials, trading parameters, and strategy details.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TradeHandlerConfig {
    /// trade handler id
    pub trade_handler_id: u32,

    /// API key used for authentication with the trading platform.
    pub api_key: String,

    /// Client code associated with the trading account.
    pub client_code: String,

    /// PIN for the client code used for additional authentication.
    pub pin: String,

    /// JWT token used for session authentication.
    pub jwt_token: String,

    /// Refresh token used to renew the session.
    pub refresh_token: String,

    /// Feed token used for accessing market data.
    pub feed_token: String,

    /// Maximum loss allowed for the trades.
    pub max_loss: f64,

    /// Maximum price allowed for the trades.
    pub max_price: f64,

    /// Maximum number of trades allowed.
    pub max_trades: u32,

    /// Symbol for the trading asset (e.g., stock ticker symbol).
    pub symbol: String,

    /// Strategy to be used for trading decisions.
    pub strategy: Strategy,

    /// Segment of the trade (e.g., equities, futures).
    pub segment: Segment,

    /// Trade sl in points
    // pub trade_sl_points: f64,
    pub sl: f64,

    /// trigger price when a trade will be executed on algorithm
    pub trigger_price: f64,

    /// quantity
    pub quantity: u32,

    /// transaction type
    pub transaction_type: TransactionType,

    ///authorization token required by Node Server
    pub auth_token: String,
}

/// A struct representing a trade signal sent from a server to execute trades.
/// This struct is used to convey trade-related information such as the strategy,
/// entry price, symbol, and transaction type to be executed in a Rust server.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TradeSignal {
    /// Strategy that triggered the trade signal
    pub strategy: Strategy,

    /// Entry price for the trade
    pub trigger_price: f64,

    /// Symbol to execute the trade on
    pub symbol: String,

    /// Transaction type (e.g., Buy or Sell)
    pub transaction_type: TransactionType,
}
/// Struct to manage client information
#[derive(Debug, Clone)]
pub struct TradeHandler {
    /// Unique trade handler id : NO CHANGE WITHOUT REQUEST
    pub trade_handler_id: u32,

    /// Client instance wrapped in Arc<Mutex> : NO CHANGE WITHOUT REQUEST
    pub client: Arc<SmartConnect>,

    /// Client profile set after initialization : NO CHANGE WITHOUT REQUEST
    pub profile: Arc<Profile>,

    /// Segment  : NO CHANGE WITHOUT REQUEST
    pub segment: Segment,

    /// Symbol which derives to  : NO CHANGE WITHOUT REQUEST
    pub symbol: String,

    /// Symbol to execute
    pub trading_symbol: String,

    /// Order Request for entry
    pub entry_req: Option<Arc<PlaceOrderReq>>,

    /// Order Response for entry
    pub entry_res: Option<Arc<PlaceOrderRes>>,

    /// Order Request for exit
    pub exit_req: Option<Arc<PlaceOrderReq>>,

    /// Order Response for exit
    pub exit_res: Option<Arc<PlaceOrderRes>>,

    /// Position details of trade
    pub position: Option<Arc<IndividualOrderStatus>>,

    /// total trades executed : NO CHANGE WITHOUT REQUEST
    pub executed_trades: u32,

    /// Max price allowed : NO CHANGE WITHOUT REQUEST
    pub max_price: f64,

    /// Max trades allowed : NO CHANGE WITHOUT REQUEST
    pub max_trades: u32,

    /// Max loss allowed : NO CHANGE WITHOUT REQUEST
    pub max_loss: f64,

    /// Strategy : NO CHANGE WITHOUT REQUEST
    pub strategy: Strategy,

    /// Trade status
    pub trade_status: TradeStatus,

    /// trigger price when a trade will be executed on algorithm
    pub trigger_price: f64,

    /// Trade entry price
    pub trade_entry_price: f64,

    /// Trade exit price
    pub trade_exit_price: f64,

    /// Trade sl in points
    // pub trade_sl_points: f64, : NO CHANGE WITHOUT REQUEST
    pub sl: f64,

    /// Trade quantity : NO CHANGE WITHOUT REQUEST
    pub quantity: u32,

    /// trade received from server after every time trade execution
    pub trade_id: u32,

    /// Trade transaction type, that is Buy or Sell : NO CHANGE WITHOUT REQUEST
    pub transaction_type: TransactionType,

    /// Exit order from admin or server : By default it will be false
    pub exit_now: bool,

    /// Wait bool = Wait till response from server after placing order
    ///  this will be immediately set to prevent repeat placing order
    pub wait: bool,

    /// Flag to check the handler is stopped or not by user
    pub stopped: bool,

    /// Server http for web server requests : NO CHANGE WITHOUT REQUEST
    pub server_http: ServerHttp,
}

/// Struct to send response after placing trade
#[derive(Debug, Serialize,Deserialize, Clone)]
pub struct NewTradeRes {
    /// Unique trade handler id : NO CHANGE WITHOUT REQUEST
    pub trade_handler_id: u32,

    /// Order Response for entry
    /// , skip_serializing_if = "Option::is_none"
    pub entry_res: Option<PlaceOrderRes>,

    /// Order Request for exit
    pub exit_req: Option<PlaceOrderReq>,

    /// Position details of trade
    pub position: Option<IndividualOrderStatus>,

    /// total trades executed : NO CHANGE WITHOUT REQUEST
    pub executed_trades: u32,

    /// Trade status
    pub trade_status: TradeStatus,

    /// Trade entry price
    pub trade_entry_price: f64,

    /// trade received from server after every time trade execution
    pub trade_id: u32,
}

/// data handler for tarde api requests
#[derive(Debug, Serialize, Deserialize)]
pub struct Payload {
    /// request type
    pub request_type: String,
    /// data as json value
    pub data: Value,
}

impl TradeHandler {
    /// TradeHandler struct implemented

    /// Creates a new `TradeHandler` instance from the provided configuration.
    ///
    /// This function initializes a `TradeHandler` using the given `TradeHandlerConfig`
    /// which contains all necessary parameters for setting up the handler.
    ///
    /// # Parameters
    ///
    /// - `config`: A `TradeHandlerConfig` struct containing the configuration details
    ///   required to create a new `TradeHandler`. This includes API credentials, trading
    ///   parameters, and strategy details.
    ///
    /// # Returns
    ///
    /// - `Result<Self>`: Returns a `Result` which is `Ok` if the `TradeHandler` was created
    ///   successfully, or an `Err` if an error occurred during initialization. The error
    ///   type is typically a string describing the issue.
    ///
    /// # Errors
    ///
    /// - Returns an error if any of the configuration details are invalid or if there is
    ///   an issue with initializing components based on the provided configuration.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let config = TradeHandlerConfig {
    ///     api_key: "your_api_key".to_string(),
    ///     client_code: "your_client_code".to_string(),
    ///     pin: "your_pin".to_string(),
    ///     jwt_token: "your_jwt_token".to_string(),
    ///     refresh_token: "your_refresh_token".to_string(),
    ///     feed_token: "your_feed_token".to_string(),
    ///     max_loss: 1000,
    ///     max_price: 150,
    ///     max_trades: 10,
    ///     symbol: "AAPL".to_string(),
    ///     strategy: Strategy::Momentum, // Assuming you have this enum
    ///     segment: Segment::Equities,   // Assuming you have this enum
    /// };
    ///
    /// match TradeHandler::new_from_session(config).await {
    ///     Ok(handler) => println!("TradeHandler created successfully"),
    ///     Err(e) => eprintln!("Error creating TradeHandler: {}", e),
    /// }
    /// ```
    pub async fn new_from_session(config: TradeHandlerConfig) -> Result<Self> {
        // let client = SmartConnect::new(
        //     config.api_key,
        //     config.client_code,
        //     config.pin,
        // ).await?;

        println!("CONFIG IS {:?}", config);

        let client = SmartConnect::new_with_session(
            config.api_key,
            config.client_code,
            config.pin,
            config.jwt_token,
            config.refresh_token,
            config.feed_token,
        )
        .await
        .unwrap();
        let profile = client.profile().await.unwrap();
        // Initialize TradeHandler with the config
        Ok(Self {
            trade_handler_id: config.trade_handler_id,
            client: Arc::new(client),
            segment: config.segment,
            symbol: config.symbol,
            trading_symbol: "".to_string(),
            entry_req: None,
            entry_res: None,
            exit_req: None,
            exit_res: None,
            position: None,
            executed_trades: 0,
            max_price: config.max_price,
            max_trades: config.max_trades,
            max_loss: config.max_loss,
            profile: Arc::new(profile),
            strategy: config.strategy,
            trade_status: TradeStatus::Closed, // Example default trade status
            trade_entry_price: 0.00,
            trade_exit_price: 0.00,
            sl: config.sl,
            trigger_price: config.trigger_price,
            quantity: config.quantity,
            trade_id: 0,
            transaction_type: config.transaction_type,
            exit_now: false,
            wait: false,
            stopped: false,
            server_http: ServerHttp::new(),
        })
    }

    /// Creating a http builder
    pub async fn create_http_client() -> Arc<Client> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap();
        Arc::new(client)
    }

    /// Performing test request
    pub async fn perform_request(
        client: Arc<Client>,
        url: &str,
        payload: serde_json::Value,
    ) -> Result<Response> {
        let response = client.post(url).json(&payload).send().await?;
        Ok(response)
    }

    /// Ltp changed and received from server
    // pub async fn ltp_changed(&mut self, ltp: f64, token: &str) {
    //     if indices_tokens.contains(&token) {
    //         println!("Indices");
    //     } else {
    //         if self.transaction_type == TransactionType::BUY && ltp <= self.sl {
    //             self.squareoff_trade().await.unwrap();
    //         } else if ltp >= self.sl {
    //             self.squareoff_trade().await.unwrap();
    //         }
    //     }
    // }

    /// Place new order
    pub async fn place_trade(&mut self) -> std::result::Result<(), String> {
        if let Some(ref order_req) = self.entry_req {
            match self.client.place_order(order_req).await {
                Ok(order_res) => {
                    self.entry_res = Some(Arc::new(order_res));

                    // let order_response = Arc::new(order_res);
                    // self.entry_res = Some(order_response.clone());
                    // if let Some(ref entry_r) = self.entry_res {
                    //     if let Some(ref uod) = entry_r.unique_order_id {
                    //         let unique_order_id = uod;
                    //         let trade_details = self
                    //             .client
                    //             .order_status(unique_order_id.clone())
                    //             .await.unwrap();
                    //         self.trade_entry_price = trade_details.order.price;
                    //         self.position = Some(Arc::new(trade_details));
                    //     }
                    // }

                    // if let Some(ref entry_res) = self.entry_res {
                    //     let order_res = &*entry_res.clone();
                    //     if let Some(ref entry_r) = order_res.unique_order_id {
                    //         let unique_order_id = entry_r.to_string();
                    //         match self.client.order_status(unique_order_id.clone()).await {
                    //             Ok(order_status) => {
                    //                 self.trade_entry_price = order_status.order.price;
                    //                 self.position = Some(Arc::new(order_status));
                    //                 return Ok(());
                    //             }
                    //             Err(e) => {
                    //                 // return Err(Box::new(e) as Error);
                    //                 // return Err(e.to_string())
                    //                 println!("Print error placing trade {:?}",e);
                    //                 return Err("No order request provided".to_string());
                    //             }
                    //         }
                    //     }
                    // }

                    Ok(())
                }
                Err(e) => Err(e.to_string()),
            }
        } else {
            Err("No Entry request provided".to_string())
        }
    }

    // pub async fn place_trade(&mut self) -> Result<Trade> {
    //     if let Some(ref order_req) = self.entry_req {
    //         match self.client.place_order(order_req).await {
    //             Ok(order_res) => {
    //                 self.entry_res = Some(Arc::new(order_res));
    //                 let mut unique_order_id: String = "".to_string();

    //                 if let Some(ref entry_res) = self.entry_res {
    //                     let order_res = &*entry_res.clone();
    //                     if let Some(ref entry_r) = order_res.unique_order_id {
    //                         unique_order_id = entry_r.to_string();
    //                         match self.client.order_status(unique_order_id.clone()).await {
    //                             Ok(order_status) => {
    //                                 self.trade_entry_price = order_status.order.price;
    //                                 self.position = Some(Arc::new(order_status));
    //                             }
    //                             Err(e) => {
    //                                 // return Err(Box::new(e) as Error);
    //                                 return Err(e);
    //                             }
    //                         }
    //                     }
    //                 }
    //                 // if let Some(ref uoid) = order_res.unique_order_id {
    //                 //     unique_order_id = uoid.to_string();
    //                 //     match self.client.order_status(unique_order_id.clone()).await {
    //                 //         Ok(order_status) => {
    //                 //             self.trade_entry_price = order_status.order.price;
    //                 //         }
    //                 //         Err(e) => {
    //                 //             // return Err(Box::new(e) as Error);
    //                 //             return Err(e);
    //                 //         }
    //                 //     }
    //                 // }

    //                 match self
    //                     .update_order("new_order", unique_order_id.clone())
    //                     .await
    //                 {
    //                     Ok(trade) => Ok(trade),
    //                     Err(e) => Err(e),
    //                 }
    //             }
    //             Err(e) => Err(e),
    //         }
    //     } else {
    //         Err("No entry request have been set...".into())
    //     }
    // }

    /// Square off trade
    pub async fn squareoff_trade(&mut self) -> std::result::Result<(), String> {
        if let Some(ref order_req) = self.exit_req {
            match self.client.place_order(&order_req).await {
                Ok(order_no_res) => {
                    self.exit_res = Some(Arc::new(order_no_res));
                    self.exit_now = false;
                }
                Err(e) => {
                    eprintln!("Failed to place order: {}", e.to_string());
                }
            }
            Ok(())
        } else {
            Err("No Exit request provided".to_string())
        }
    }

    /// Easily access entry response
    pub fn get_entry_res(&self) -> Option<Arc<PlaceOrderRes>> {
        self.entry_res.clone()
    }

    /// send new order request
    pub async fn update_order(&self, request_type: &str, unique_order_id: String) -> Result<Trade> {
        if request_type == "new_order" {
            let client = Client::new();
            let url = server_utils::end_points::order_url(); // Replace with your actual URL

            // if self.position.is_some() {
            if let Some(_position) = self.position.clone() {
                let client_code = self.client.client_code.clone();
                let payload = Payload {
                    request_type: "new_order".to_string(),
                    data: serde_json::json!({"client_code":client_code,"unique_order_id":unique_order_id}),
                };
                let response = client.post(url).json(&payload).send().await;

                match response {
                    Ok(res) => {
                        // Check if the response status is successful
                        if res.status().is_success() {
                            let trade = res.json::<Trade>().await.unwrap();
                            println!("Trade: {:?}", trade);
                            Ok(trade)
                        } else {
                            // Handle non-successful status codes
                            // println!("Failed to send order status. HTTP Status: {}", res.status());
                            Err("Failed to send order status. HTTP Status:".into())
                        }
                    }
                    Err(e) => {
                        // Handle the error that occurred during the request
                        eprintln!("Request error: {:?}", e);
                        Err(Box::new(e))
                    }
                }
            } else {
                println!("Position not initiated...");
                Ok(Trade {
                    trade_id: 0,
                    status: "".to_string(),
                    comment: "".to_string(),
                })
            }
        } else if request_type == "exit_order" {
            println!("Invalid request type");
            Ok(Trade {
                trade_id: 0,
                status: "".to_string(),
                comment: "".to_string(),
            })
        } else {
            Ok(Trade {
                trade_id: 0,
                status: "".to_string(),
                comment: "".to_string(),
            })
        }
    }
}
