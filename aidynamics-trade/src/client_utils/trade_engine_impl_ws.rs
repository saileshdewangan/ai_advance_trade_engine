// use aidynamics_trade_utils::Error;
// use tokio::sync::Mutex;
// use reqwest::{Client, Response};
use serde_json::{from_value, Result as SerdeResult, Value};

use crate::{
    order::{OrderSetter, PlaceOrderReq},
    types::{ExchangeType, ProductType, TransactionType},
    user::Profile,
    websocket::{angel_one_websocket::SubscriptionExchange, websocket_client::Signal},
    // Result,
};

use serde_repr::{Deserialize_repr, Serialize_repr};
use tokio::sync::mpsc;

const INDICES_TOKENS: [&'static str; 3] = ["26009", "26000", "26037"];

/// Enum trade status
#[derive(Serialize_repr, Deserialize_repr, Debug, PartialEq, Clone)]
#[repr(u8)]
pub enum TradeStatus {
    /// Variant showing running status
    Open = 1,
    /// Variant showing stopped status
    Closed = 0,
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
    StochasticRsi = 1,

    /// Stochastic + RSI + MACD
    // #[serde(rename = "rsi_macd_stochastic")]
    RsiMacdStochastic = 2,

    /// EMA-5
    // #[serde(rename = "5_ema")]
    Ema5 = 3,

    /// Bollinger Band
    // #[serde(rename = "bollinger_band")]
    BollingerBand = 4,
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
    /// Unique trade handler id : NO CHANGE WITHOUT REQUEST
    pub trade_engine_id: u32,

    /// trade received from server after every time trade execution
    pub trade_id: u32,
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

    /// Client profile set after initialization : NO CHANGE WITHOUT REQUEST
    pub profile: Option<Profile>,

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

    /// Trailing sl
    #[serde(default = "default_trailing_sl")]
    pub trailing_sl: f32,

    /// Target in points
    #[serde(default = "default_target")]
    pub target: f32,

    /// Trade quantity : NO CHANGE WITHOUT REQUEST
    pub quantity: u32,

    #[serde(rename = "tradeid")]
    /// trade received from server after every time trade execution
    pub trade_id: u32,

    /// Trade transaction type, that is Buy or Sell : NO CHANGE WITHOUT REQUEST
    pub transaction_type: TransactionType,

    /// This flag is to pause tracking operation on open status
    /// By default this is false
    #[serde(default = "default_pause")]
    pub pause: bool,

    /// Exit order from admin or server : By default it will be false
    // #[serde(default = "default_exit_now")]
    // pub exit_now: bool,

    /// Wait bool = Wait till response from server after placing order
    /// this will be immediately set to prevent repeat placing order
    #[serde(default = "default_wait")]
    pub wait: bool,
    // Flag to check the engine is stopped or not by user
    // pub engine_status: EngineStatus,
}

// Default value provider for `trading_symbol`
fn default_trading_symbol() -> String {
    String::from("")
}

// Default value provider for `symbol_token`
fn default_symbol_token() -> String {
    String::from("")
}

// Default value provider for `exit_now`
fn default_exit_now() -> bool {
    false
}

// Default value provider for `wait`
fn default_wait() -> bool {
    false
}

// Default value provider for `exchange type`
fn default_exchange_type() -> ExchangeType {
    ExchangeType::NFO
}

// Default value provider for `segment`
fn default_segment() -> SubscriptionExchange {
    SubscriptionExchange::NSEFO
}

// Default value provider for `trade_id`
fn default_trade_id() -> u32 {
    0
}

fn default_trailing_sl() -> f32 {
    0.0
}

fn default_target() -> f32 {
    0.0
}

// Default value provider for `executed_trades`
fn default_executed_trades() -> u32 {
    0
}

// Default value for pause flag
fn default_pause() -> bool {
    false
}

impl Default for TradeEngine {
    fn default() -> Self {
        TradeEngine {
            trade_engine_id: 0,
            client_id: 0,
            profile: None,
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
            sl: 0.0,
            trailing_sl: 0.0,
            target: 0.0,
            quantity: 0,
            trade_id: 0,
            transaction_type: TransactionType::BUY,
            wait: false,
            pause: false,
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
    pub async fn execute_trade(&mut self, tx_ws: mpsc::Sender<Signal>) {
    pub async fn execute_trade(&mut self, tx_ws: mpsc::Sender<Signal>) {
        self.wait = true;
        if let Some(req) = self.entry_req.clone() {
            tx_ws.send(Signal::ExecuteTrade(req)).await.unwrap();
        }
    }

    /// New trade function to place new order with params
    pub async fn squareoff_trade(&mut self, tx_ws: mpsc::Sender<Signal>) {
        self.wait = true;
        if let Some(req) = self.exit_req.clone() {
            tx_ws
                .send(Signal::SquareOffTrade {
                    client_id: self.client_id,
                    order_req: req,
                    trade_id: self.trade_id,
                })
                .await
                .unwrap();
        }
    }

    /// New trade function to place new order with params
    pub async fn trade_not_exist(&mut self, tx_ws: mpsc::Sender<Signal>) {
        self.wait = true;
        if let Some(req) = self.exit_req.clone() {
            tx_ws
                .send(Signal::SquareOffTrade {
                    client_id: self.client_id,
                    order_req: req,
                    trade_id: self.trade_id,
                })
                .await
                .unwrap();
        }
    }

    /// Prepare entry request
    pub async fn prepare_entry_req(&mut self) {
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

    /// Prepare entry request
    pub async fn prepare_exit_req(&mut self) {
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
}
