use std::sync::Arc;

use chrono::{Duration, Utc};
// use aidynamics_trade_utils::Error;
// use tokio::sync::Mutex;
// use reqwest::{Client, Response};
use serde_json::{from_value, Result as SerdeResult, Value};

use crate::{
    order::{OrderSetter, PlaceOrderReq},
    redis_utils::Signal,
    types::{ExchangeType, ProductType, TransactionType},
    websocket::angel_one_websocket::SubscriptionExchange,
};

use serde_repr::{Deserialize_repr, Serialize_repr};
use tokio::sync::mpsc;

/// Enum trade status
#[derive(Serialize_repr, Deserialize_repr, Debug, PartialEq, Clone)]
#[repr(u8)]
pub enum TradeStatus {
    /// Variant showing running status
    Open = 1,
    /// Variant showing stopped status
    Closed = 0,
    /// variant showing waiting to match price and execute new trade
    Pending = 2,
    /// Variant showing sl or target hit
    Triggered = 3,
    /// Variant showing price matched and confirming from server
    Confirming = 4,
    /// Variant showing waiting to for acknowledge
    AwaitingConfirmation = 5,
}

/// Enum engine status
#[derive(Serialize_repr, Deserialize_repr, Debug, PartialEq, Clone)]
#[repr(u8)]
pub enum EngineStatus {
    /// Variant showing running status
    Running = 1,
    /// Variant showing stopped status
    Stopped = 0,
}

/// Enum to strategy
#[derive(Debug, Serialize_repr, Deserialize_repr, PartialEq, Eq, Clone)]
#[repr(u8)]
pub enum Strategy {
    /// Stochastic RSI
    // #[serde(rename = "stochastic_rsi")]
    // StochasticRsi = 1,
    Momentum = 1, // Stochastic RSI

    /// Stochastic + RSI + MACD
    // #[serde(rename = "rsi_macd_stochastic")]
    // RsiMacdStochastic = 2,
    RangeBreak = 2, // Rsi Macd Stochastic

    /// EMA-5
    // #[serde(rename = "5_ema")]
    // Ema5 = 3,
    MomentumShift = 3, // Ema5

    /// Bollinger Band
    // #[serde(rename = "bollinger_band")]
    // BollingerBand = 4,
    DynamicChannel = 4, // Bollinger Band
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

#[derive(Debug, Serialize, Deserialize, Clone)]
/// Trade response return by Web Server after placing trade
pub struct TradeRes {
    /// Unique trade self id : NO CHANGE WITHOUT REQUEST
    pub trade_engine_id: u32,

    /// trade received from server after every time trade execution
    pub trade_id: u32,

    /// Execution price
    pub price: f32,

    /// To identify which stategy is processing
    pub strategy: Strategy,
}

impl TradeRes {
    /// Build trade engine from data provided
    pub fn from_data(data: &Value, trade_engine_id: u32, strategy: Strategy) -> Option<TradeRes> {
        if let (Some(trade_id), Some(price)) = (
            data.get("trade_id").and_then(|v| v.as_u64()),
            data.get("price").and_then(|v| v.as_f64()),
        ) {
            Some(TradeRes {
                trade_engine_id,
                trade_id: trade_id as u32,
                price: price as f32,
                strategy,
            })
        } else {
            None
        }
    }
}

// Struct to hold the updates for TradeEngine fields
#[derive(Debug, Serialize, Deserialize, Clone)]
/// `TradeEngineUpdates` is a configuration struct used to update specific trading parameters
/// within a trade engine. It provides customizable settings for managing trade limits,
/// risk controls, and strategy parameters.
///
/// This struct is typically used to update an existing `TradeEngine` instance with new values
/// for fields like transaction type, stop-loss thresholds, trade targets, and limits. Each
/// field represents a specific aspect of the trading strategy, allowing for fine-tuned control
/// over trade execution and risk management.
pub struct TradeEngineUpdates {
    /// Specifies the type of transaction (e.g., `Buy` or `Sell`) to be executed.
    #[serde(rename = "transaction_type")]
    pub transaction_type: TransactionType,

    /// Sets the maximum number of trades allowed for the current session or strategy.
    /// This limit helps control trade frequency and prevent overtrading.
    pub max_trades: u32,

    /// Defines the maximum allowable loss for the current trade. If the loss exceeds this
    /// threshold, the trade engine may automatically close the position to mitigate risk.
    pub max_loss: f32,

    /// Sets the trailing stop-loss value, which dynamically adjusts the stop-loss as the
    /// trade becomes profitable. This helps lock in profits while allowing for favorable
    /// price movement.
    pub trailing_sl: f32,

    /// Specifies the target price at which the trade should be exited to realize profits.
    /// This value helps automate profit-taking decisions within the strategy.
    pub target: f32,

    /// Defines the trading strategy to be applied for the trade execution.
    /// The `Strategy` enum or type provides various strategy options (e.g., `Scalping`, `Swing`).
    pub strategy: Strategy,

    /// Sets the initial stop-loss value for the trade. If the trade's price reaches this level,
    /// the position will be exited to minimize losses.
    pub sl: f32,

    /// Specifies the quantity of the asset to be traded. This value determines the position size
    /// and directly impacts the trade's potential risk and reward.
    pub quantity: u32,

    /// Sets the maximum allowable price for trade entry. If the asset's price exceeds this limit,
    /// the trade will not be executed, helping avoid trades at unfavorable price levels.
    pub max_price: f32,

    /// Symbol which derives to
    #[serde(default = "default_symbol")]
    pub symbol: String,
}

fn default_symbol() -> String {
    "Nifty".to_string()
}

/// Struct to manage client information
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TradeEngine {
    /// Unique trade engine id : NO CHANGE WITHOUT REQUEST
    pub trade_engine_id: u32,

    /// client id
    #[serde(rename = "client_id")]
    pub client_id: u32,

    // /// Client profile set after initialization : NO CHANGE WITHOUT REQUEST
    // pub profile: Option<Profile>,
    /// Segment  : NO CHANGE WITHOUT REQUEST, this is saved as segment but it is subscriptionExchange
    #[serde(default = "default_segment")]
    pub segment: SubscriptionExchange,

    /// Symbol which derives to  : NO CHANGE WITHOUT REQUEST
    pub symbol: String,

    /// Symbol to execute
    #[serde(default = "default_trading_symbol")]
    pub trading_symbol: String,

    /// Symbol to execute
    #[serde(default = "default_exchange_type")]
    pub exchange_type: ExchangeType,

    /// Symbol token
    #[serde(default = "default_symbol_token")]
    pub symbol_token: String,

    /// Order Request for entry
    pub entry_req: Option<PlaceOrderReq>,

    /// Order Request for exit
    pub exit_req: Option<PlaceOrderReq>,

    // /// Order Request for entry TESTING : IT HAS TO BE REMOVED
    // pub entry_req_test: Option<PlaceOrderReq>,

    // /// Order Request for exit TESTING : IT HAS TO BE REMOVED
    // pub exit_req_test: Option<PlaceOrderReq>,
    /// total trades executed : NO CHANGE WITHOUT REQUEST
    #[serde(default = "default_executed_trades")]
    pub executed_trades: u32,

    /// Max price allowed : NO CHANGE WITHOUT REQUEST
    pub max_price: f32,

    /// Max trades allowed : NO CHANGE WITHOUT REQUEST
    pub max_trades: u32,

    /// Max loss allowed : NO CHANGE WITHOUT REQUEST
    pub max_loss: f32,

    /// Strategy : NO CHANGE WITHOUT REQUEST
    pub strategy: Strategy,

    /// Trade status
    pub trade_status: TradeStatus,

    /// trigger price when a trade will be executed on algorithm
    pub trigger_price: f32,

    #[serde(rename = "entryprice")]
    /// Trade entry price
    pub trade_entry_price: f32,

    /// Trade exit price
    // pub trade_exit_price: f32,

    /// Trade sl in points
    // pub trade_sl_points: f64, : NO CHANGE WITHOUT REQUEST
    pub sl: f32,

    /// Trade sl price, which will be calculated at time of execution
    #[serde(default = "default_stop_loss_price")]
    pub stop_loss_price: f32,

    /// Trailing sl
    #[serde(default = "default_trailing_sl")]
    pub trailing_sl: f32,

    /// Target in points
    #[serde(default = "default_target")]
    pub target: f32,

    /// Trade target price, which will be calculated at time of execution
    #[serde(default = "default_target_price")]
    pub target_price: f32,

    /// Trade quantity : NO CHANGE WITHOUT REQUEST
    pub quantity: u32,

    #[serde(rename = "tradeid")]
    /// trade received from server after every time trade execution
    pub trade_id: u32,

    /// Trade transaction type, that is Buy or Sell : NO CHANGE WITHOUT REQUEST
    pub transaction_type: TransactionType,

    /// Open position type, that is to Long or Short the instrument
    #[serde(default = "default_position_type")]
    pub position_type: TransactionType,

    /// Execution time
    #[serde(default = "default_execution_time")]
    pub execution_time: i64,

    /// Is margin available
    #[serde(default = "default_margin")]
    pub margin: i32,
}

/// Struct to hold information received from server to open new trade
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NewTrade {
    /// Symbol which derives to  : NO CHANGE WITHOUT REQUEST
    pub symbol: String,

    /// Trade engine
    // pub trade_engine_id: u32,

    /// Symbol to execute
    #[serde(default = "default_trading_symbol")]
    pub buyer_trading_symbol: String,

    /// Symbol token
    #[serde(default = "default_symbol_token")]
    pub buyer_symbol_token: String,

    /// Symbol to execute
    #[serde(default = "default_trading_symbol")]
    pub seller_trading_symbol: String,

    /// Symbol token
    #[serde(default = "default_symbol_token")]
    pub seller_symbol_token: String,

    /// Symbol to execute
    #[serde(default = "default_exchange_type")]
    pub exchange_type: ExchangeType,

    /// Strategy : NO CHANGE WITHOUT REQUEST
    pub strategy: Strategy,

    /// trigger price when a trade will be executed on algorithm
    pub trigger_price: f32,

    /// Position type, Long or Short to open
    pub position_type: TransactionType,

    /// Trade status to initialize
    pub trade_status: TradeStatus,
}

// Default value provider for `trading_symbol`
fn default_trading_symbol() -> String {
    String::from("")
}

// Default value provider for `symbol_token`
fn default_symbol_token() -> String {
    String::from("")
}

// Default value provider for `exchange type`
fn default_exchange_type() -> ExchangeType {
    ExchangeType::NFO
}

// Default value provider for `segment`
fn default_segment() -> SubscriptionExchange {
    SubscriptionExchange::NSEFO
}

fn default_trailing_sl() -> f32 {
    0.0
}

fn default_target() -> f32 {
    0.0
}

fn default_target_price() -> f32 {
    0.0
}

// Default value provider for `executed_trades`
fn default_executed_trades() -> u32 {
    0
}

// Default stop loss price flag
fn default_stop_loss_price() -> f32 {
    0.00
}

// Default value for position type flag
fn default_position_type() -> TransactionType {
    TransactionType::BUY
}

fn default_execution_time() -> i64 {
    Utc::now().timestamp()
}

fn default_margin() -> i32 {
    0
}

impl Default for TradeEngine {
    fn default() -> Self {
        TradeEngine {
            trade_engine_id: 0,
            client_id: 0,
            // profile: None,
            segment: SubscriptionExchange::NSEFO,
            symbol: "".to_string(),
            trading_symbol: "".to_string(),
            exchange_type: ExchangeType::NFO,
            symbol_token: "".to_string(),
            entry_req: None,
            exit_req: None,
            executed_trades: 0,
            max_price: 0.0,
            max_trades: 0,
            max_loss: 0.0,
            strategy: Strategy::Momentum,
            trade_status: TradeStatus::Closed,
            trigger_price: 0.0,
            trade_entry_price: 0.0,
            stop_loss_price: 0.0,
            sl: 0.0,
            trailing_sl: 0.0,
            target: 0.0,
            target_price: 0.0,
            quantity: 0,
            trade_id: 0,
            transaction_type: TransactionType::BUY,
            position_type: TransactionType::BUY,
            execution_time: 0,
            margin: 0,
        }
    }
}

impl TradeEngine {
    /// TradeEngine struct implemented

    /// Creates a new `TradeEngine` instance from the provided configuration.
    ///
    /// This function initializes a `TradeEngine` using the given `TradeEngineConfig`
    /// which contains all necessary parameters for setting up the engine.
    ///
    /// # Parameters
    ///
    /// - `config`: A `TradeEngineConfig` struct containing the configuration details
    ///   required to create a new `TradeEngine`. This includes API credentials, trading
    ///   parameters, and strategy details.
    ///
    /// # Returns
    ///
    /// - `Result<Self>`: Returns a `Result` which is `Ok` if the `TradeEngine` was created
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
    /// let config = TradeEngineConfig {
    ///     max_loss: 1000,
    ///     max_price: 150,
    ///     max_trades: 10,
    ///     symbol: "AAPL".to_string(),
    ///     strategy: Strategy::Momentum, // Assuming you have this enum
    ///     segment: Segment::Equities,   // Assuming you have this enum
    /// };
    ///
    /// match TradeEngine::new_from_session(config).await {
    ///     Ok(engine) => println!("TradeEngine created successfully"),
    ///     Err(e) => eprintln!("Error creating TradeEngine: {}", e),
    /// }
    /// ```

    /// Function to create TradeEngine from serde_json::Value
    pub async fn create_trade_engine(data: Value) -> SerdeResult<TradeEngine> {
        from_value::<TradeEngine>(data)
    }

    /// New trade function to place new order with params
    pub async fn execute_trade(&mut self, tx_order_processor: Arc<mpsc::Sender<Signal>>) {
        println!("Trade executed ...");
        if let Some(req) = self.entry_req.clone() {
            self.trade_status = TradeStatus::Confirming;
            if let Err(err) = tx_order_processor
                .send(Signal::ExecuteOrder {
                    order_req: req,
                    strategy: self.strategy.clone(),
                    trade_engine_id: self.trade_engine_id,
                    client_id: self.client_id,
                })
                .await
            {
                tracing::error!("Failed to send trade execution signal: {:?}", err);
            }
        }
    }

    /// Checks condition can it accept new trade according to condition or not
    pub fn can_accept_new_trade(&self, strategy: &Strategy) -> bool {
        print!(
            "\nStrategy can accept {:?}, Engine id {:?}, status {:?}, executed {:?}, max {:?}, client_id {:?}",
            self.strategy,
            self.trade_engine_id,
            self.trade_status,
            self.executed_trades,
            self.max_trades,
            self.client_id
        );
        if self.client_id != 0 {
            self.executed_trades < self.max_trades
                && self.trade_status == TradeStatus::Closed
                && self.strategy == *strategy
        } else {
            self.executed_trades < self.max_trades
                && self.trade_status == TradeStatus::Closed
                && self.strategy == *strategy
        }
    }

    /// Trailing stop loss function by 5 points
    pub fn trail_stop_loss(&mut self, current_price: f32) {
        if self.transaction_type == TransactionType::BUY {
            if current_price >= self.stop_loss_price + 5.0 {
                self.stop_loss_price += 5.0;
            }
        } else if self.transaction_type == TransactionType::SELL {
            if current_price <= self.stop_loss_price - 5.0 {
                // self.stop_loss_price -= 5.0;
                println!(
                    "SELL Trailing stop loss updated to"
                )
            }
        }
    }

    /// Update variants on new trade open
    pub async fn update_from(&mut self, other: &NewTrade) {
        self.exchange_type = other.exchange_type;
        self.trigger_price = other.trigger_price;
        self.position_type = other.position_type;

        match self.transaction_type {
            TransactionType::BUY => {
                self.symbol_token = other.buyer_symbol_token.clone();
                self.trading_symbol = other.buyer_trading_symbol.clone();
            }
            TransactionType::SELL => {
                self.symbol_token = other.seller_symbol_token.clone();
                self.trading_symbol = other.seller_trading_symbol.clone();
            }
        }

        self.trade_status = other.trade_status.clone(); //TradeStatus::Pending;

        // Call the method to prepare entry
        self.prepare_entry_req().await;

        self.prepare_exit_req().await;
    }

    /// Update variants when trades are opens
    pub async fn update_values(&mut self) {
        match self.transaction_type {
            TransactionType::BUY => {
                println!("Target in BUY -> {:?}", self.target);
                self.target_price = if self.target == 0.0 {
                    self.trade_entry_price + 120000.0
                } else {
                    self.trade_entry_price + self.target
                };
                self.stop_loss_price = self.trade_entry_price - self.sl;
            }
            TransactionType::SELL => {
                println!("Target in SELL -> {:?}", self.target);
                self.target_price = if self.target != 0.0 {
                    self.trade_entry_price - self.target
                } else {
                    0.0
                };
                self.stop_loss_price = self.sl - self.trade_entry_price;
            }
        }
    }

    /// Disconnect and reset the variants values
    pub async fn disconnect(&mut self) {
        self.trade_status = TradeStatus::Closed;
        self.entry_req = None;
        self.exit_req = None;
        self.trade_id = 0;
        self.execution_time = 0
    }

    /// square off trade function to place new order with params
    pub async fn squareoff_trade(
        &mut self,
        tx_order_processor: Arc<mpsc::Sender<Signal>>,
        remove_trade_engine: bool,
    ) {
        // if remove_trade_engine == false {
        //     self.trade_status = TradeStatus::Closed;
        // }
        if let Some(req) = self.exit_req.clone() {
            let strategy_clone = self.strategy.clone();
            if let Err(err) = tx_order_processor
                .send(Signal::SquareOffTrade {
                    client_id: self.client_id,
                    order_req: req,
                    trade_id: self.trade_id,
                    trade_engine_id: self.trade_engine_id,
                    remove_trade_engine,
                    strategy: strategy_clone,
                })
                .await
            {
                tracing::error!("Failed to send trade execution signal: {:?}", err);
            }
        }
    }

    // **PRODUCTION CODE - DO NOT MODIFY UNLESS NECESSARY : BEGIN**

    /// Prepare entry request
    // pub async fn prepare_entry_req(&mut self) {
    //     println!("Trading symbol = {:?}", self.trading_symbol);
    //     let entry_req = PlaceOrderReq::new(
    //         self.trading_symbol.clone(),
    //         self.symbol_token.clone(),
    //         self.transaction_type,
    //     )
    //     .exchange(self.exchange_type)
    //     .quantity(self.quantity)
    //     .product_type(ProductType::IntraDay);
    //     self.entry_req = Some(entry_req);
    // }

    // /// Prepare exit request
    // pub async fn prepare_exit_req(&mut self) {
    //     let exit_req = PlaceOrderReq::new(
    //         self.trading_symbol.clone(),
    //         self.symbol_token.clone(),
    //         if self.transaction_type == TransactionType::BUY {
    //             TransactionType::SELL
    //         } else {
    //             TransactionType::BUY
    //         },
    //     )
    //     .exchange(self.exchange_type)
    //     .quantity(self.quantity)
    //     .product_type(ProductType::IntraDay);
    //     self.exit_req = Some(exit_req);
    // }

    // /// Reset the values
    // pub async fn reset(&mut self) {
    //     self.trade_status = TradeStatus::Closed;
    //     self.exchange_type = ExchangeType::NFO;
    //     self.symbol_token = String::from("");
    //     self.trigger_price = 0.0;
    //     self.trading_symbol = String::from("");
    //     self.stop_loss_price = 0.0;
    // }

    // **PRODUCTION CODE - DO NOT MODIFY UNLESS NECESSARY : END**

    pub async fn prepare_entry_req(&mut self) {
        // println!("Trading symbol = {:?}", self.trading_symbol);
        // let entry_req = PlaceOrderReq::new("ICICIBANK-EQ", "4963", self.position_type)
        //     .exchange(ExchangeType::NSE)
        //     .quantity(1)
        //     .product_type(ProductType::IntraDay);
        // self.entry_req = Some(entry_req);

        let entry_req = PlaceOrderReq::new(
            self.trading_symbol.clone(),
            self.symbol_token.clone(),
            self.transaction_type,
        )
        .exchange(self.exchange_type)
        .quantity(self.quantity)
        .product_type(ProductType::IntraDay);
        self.entry_req = Some(entry_req);
    }

    /// Prepare exit request
    pub async fn prepare_exit_req(&mut self) {
        // let exit_req = PlaceOrderReq::new(
        //     "ICICIBANK-EQ",
        //     "4963",
        //     if self.position_type == TransactionType::BUY {
        //         TransactionType::SELL
        //     } else {
        //         TransactionType::BUY
        //     },
        // )
        // .exchange(ExchangeType::NSE)
        // .quantity(1)
        // .product_type(ProductType::IntraDay);
        // self.exit_req = Some(exit_req);

        let exit_req = PlaceOrderReq::new(
            self.trading_symbol.clone(),
            self.symbol_token.clone(),
            if self.transaction_type == TransactionType::BUY {
                TransactionType::SELL
            } else {
                TransactionType::BUY
            },
        )
        .exchange(self.exchange_type)
        .quantity(self.quantity)
        .product_type(ProductType::IntraDay);
        self.exit_req = Some(exit_req);
    }

    /// Reset the values
    pub async fn reset(&mut self) {
        self.trade_status = TradeStatus::Closed;
        self.exchange_type = ExchangeType::NFO;
        self.symbol_token = String::from("");
        self.trigger_price = 0.0;
        self.trading_symbol = String::from("");
        self.stop_loss_price = 0.0;
        self.execution_time = 0;
    }

    /// Compares the execution_time in this MyData struct with the current time.
    /// Returns true if the difference is greater than 3 hours, false otherwise.
    pub fn exceeds_threshold(&self) -> bool {
        let now_epoch = Utc::now().timestamp();
        let difference = Duration::seconds(now_epoch - self.execution_time);
        let three_hours = Duration::hours(3);
        difference > three_hours
    }
}
