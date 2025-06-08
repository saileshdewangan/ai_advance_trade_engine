#![allow(warnings)]
use std::sync::Arc;
use std::vec;
// use std::{array, f64::consts::E, fmt::write, intrinsics::mir::place, iter, str::FromStr};

use crate::types::ExchangeType;
use crate::websocket::angel_one_websocket::{
    SubscriptionBuilder, SubscriptionExchange, SubscriptionMode,
};

// use crate::websocket::websocket_client::{Signal, WebSocketClient};
use crate::redis_utils::Signal;
use crate::{
    // order::{OrderSetter, PlaceOrderReq},
    trade_engine::TradeEngine,
    types::TransactionType,
};

// use futures::channel::mpsc::Receiver;
// use tokio::sync::mpsc::Receiver;

use redis::AsyncCommands;
use serde_json::json;

// use websocket_client::WebSocketClient;
use crate::trade_engine::{NewTrade, Strategy, TradeStatus};
use tokio::sync::mpsc::Sender;

// use futures_util::{SinkExt, StreamExt};
// use tokio::sync::mpsc;
// use tokio_tungstenite::tungstenite::protocol::Message;
// use tokio_tungstenite::connect_async;
// use redis::AsyncCommands;

/// Get indices instrument name by token
pub fn get_instrument(token: &str) -> Instrument {
    match token {
        "26000" => Instrument::NIFTY,
        "26009" => Instrument::BANKNIFTY,
        "26037" => Instrument::FINNIFTY,
        "19000" => Instrument::SENSEX,

        _ => Instrument::Other,
    }
}

/// Instrument types
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub enum Instrument {
    /// Nifty index
    NIFTY,
    /// Banknifty index
    BANKNIFTY,
    /// Finnifty index
    FINNIFTY,
    /// Sensex index
    SENSEX,
    /// Any other stock || option || future
    Other,
}

impl ToString for Instrument {
    fn to_string(&self) -> String {
        match self {
            Instrument::NIFTY => "NIFTY".to_string(),
            Instrument::BANKNIFTY => "BANKNIFTY".to_string(),
            Instrument::FINNIFTY => "FINNIFTY".to_string(),
            Instrument::SENSEX => "SENSEX".to_string(),
            Instrument::Other => "Other".to_string(),
        }
    }
}

impl Instrument {
    /// Determines if the instrument is an index
    fn is_index(&self) -> bool {
        matches!(
            self,
            Instrument::NIFTY | Instrument::BANKNIFTY | Instrument::FINNIFTY | Instrument::SENSEX
        )
    }
}

/// `EngineProcessor` encapsulates the logic for processing signals related to a specific strategy.
/// It holds all the necessary components for receiving signals, interacting with external services
/// (Redis, AngelOne), and sending signals to other parts of the application.
#[derive(Debug, Clone)]
pub struct EngineProcessor {
    /// The strategy that this processor is responsible for.
    strategy_to_process: Strategy,
    /// Sender for broadcasting signals to multiple receivers.
    tx_broadcast: Arc<tokio::sync::broadcast::Sender<Signal>>,
    /// Sender for sending signals to the main application thread.
    tx_main: Sender<Signal>,
    /// Sender for sending signals to the Redis service.
    tx_redis: Sender<Signal>,
    /// Redis client for interacting with the Redis database.
    redis_client: redis::Client,
    /// Sender for sending signals to the AngelOne service.
    tx_angelone_sender: Sender<Signal>,
    /// Sender for sending signals to the order processing component.
    tx_order_processor: Sender<Signal>,
}

impl EngineProcessor {
    /// Creates a new `EngineProcessor` instance.
    ///
    /// # Arguments
    ///
    /// * `strategy_to_process` - The strategy that this processor will handle.
    /// * `rx_strategy` - The boradcast receiver for signals related to the strategy.
    /// * `tx_broadcast` - The sender for broadcasting signals.
    /// * `tx_main` - The sender for signals to the main application.
    /// * `tx_redis_clone` - The sender for signals to Redis.
    /// * `redis_client` - The Redis client.
    /// * `tx_angelone_sender` - The sender for signals to AngelOne.
    ///
    /// # Returns
    ///
    /// A new `EngineProcessor` instance.
    pub fn new(
        strategy_to_process: Strategy,
        tx_broadcast: Arc<tokio::sync::broadcast::Sender<Signal>>,
        tx_main: Sender<Signal>,
        tx_redis: Sender<Signal>,
        redis_client: redis::Client,
        tx_angelone_sender: Sender<Signal>,
        tx_order_processor: Sender<Signal>,
    ) -> Self {
        Self {
            strategy_to_process,
            tx_broadcast,
            tx_main,
            tx_redis,
            redis_client,
            tx_angelone_sender,
            tx_order_processor,
        }
    }

    /// Processes incoming signals related to the strategy.
    ///
    /// This method continuously receives signals from the `rx_strategy` receiver and
    /// processes them according to the strategy's logic.
    pub async fn process_engine(
        &mut self,
        mut rx_strategy: tokio::sync::broadcast::Receiver<Signal>,
    ) {
        let mut trade_handlers: Vec<TradeEngine> = Vec::new();

        while let msg = rx_strategy.recv().await.unwrap() {
            // Process the message
            // println!("\n\nMessage is {:?}", msg);
            match msg {
                Signal::PriceFeed { token, ltp } => {
                    // println!("\n {:?}, {:?}", token, ltp);
                    let instrument = get_instrument(&token);
                    for handler in trade_handlers.iter_mut() {
                        match instrument {
                            Instrument::NIFTY
                            | Instrument::BANKNIFTY
                            | Instrument::FINNIFTY
                            | Instrument::SENSEX => {
                                if handler.trade_status == TradeStatus::Pending
                                    && handler.symbol == instrument.to_string()
                                    && handler.strategy == self.strategy_to_process
                                {
                                    match handler.position_type {
                                        TransactionType::BUY => {
                                            // handle_buy_signal(handler, ltp, tx_redis_clone.clone())
                                            //     .await
                                            self.handle_buy_signal(handler, ltp).await
                                        }
                                        TransactionType::SELL => {
                                            // handle_sell_signal(handler, ltp, tx_redis_clone.clone())
                                            //     .await
                                            self.handle_sell_signal(handler, ltp).await
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            Instrument::Other => {
                                if handler.symbol_token == token
                                    && handler.trade_status == TradeStatus::Open
                                {
                                    if handler.transaction_type == TransactionType::BUY {
                                        println!(
                                            "Buy Pnl : {:?},Symbol : {:?}, Entry : {:?}, Ltp : {:?}",
                                            (ltp - handler.trade_entry_price) * handler.quantity as f32,
                                            handler.trading_symbol,
                                            handler.trade_entry_price,
                                            ltp,
                                        );

                                        if ltp <= handler.stop_loss_price
                                            || ltp >= handler.target_price
                                        // (handler.trade_entry_price - handler.sl)
                                        {
                                            handler.trade_status = TradeStatus::Triggered;
                                            // handler
                                            //     .squareoff_trade(&tx_redis_clone.clone(), false)
                                            //     .await;
                                            handler
                                                .squareoff_trade(
                                                    self.tx_order_processor.clone(),
                                                    false,
                                                )
                                                .await;
                                            println!("\n\nBuy square off trade");
                                        }
                                    } else {
                                        println!(
                                            "Sell Pnl : {:?}, Target price = {:?}, Sl price = {:?}",
                                            (handler.trade_entry_price - ltp)
                                                * handler.quantity as f32,
                                            handler.target_price,
                                            handler.stop_loss_price
                                        );

                                        if ltp >= handler.stop_loss_price
                                            || ltp <= handler.target_price
                                        // (handler.sl - handler.trade_entry_price)
                                        {
                                            handler.trade_status = TradeStatus::Triggered;
                                            handler
                                                .squareoff_trade(
                                                    self.tx_order_processor.clone(),
                                                    false,
                                                )
                                                .await;
                                            println!("\n\nSell square off trade");
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Signal::InitializeOpenTrade {
                    new_trade,
                    trade_res,
                } => {
                    if (new_trade.strategy == self.strategy_to_process) {
                        if let Some(existing_engine) = trade_handlers.iter_mut().find(|existing| {
                            existing.trade_engine_id == trade_res.trade_engine_id
                                && existing.can_accept_new_trade(&self.strategy_to_process)
                        }) {
                            println!("\nNew trade initialized {:?}, {:?}", new_trade, trade_res);

                            existing_engine.update_from(&new_trade).await;

                            let subscription = SubscriptionBuilder::new("abcde12345")
                                .mode(SubscriptionMode::Ltp)
                                .subscribe(
                                    SubscriptionExchange::NSEFO,
                                    vec![existing_engine.symbol_token.as_str()],
                                )
                                .build();

                            if (existing_engine.trade_engine_id == trade_res.trade_engine_id) {
                                existing_engine.trade_entry_price = trade_res.price;
                                existing_engine.update_values().await;

                                existing_engine.trade_id = trade_res.trade_id;
                                existing_engine.prepare_exit_req();
                            }

                            if let Err(e) = self
                                .tx_angelone_sender
                                .send(Signal::Subscribe(subscription))
                                .await
                            {
                                eprintln!("\nFailed to send subscription signal: {:?}", e);
                            }

                            println!("\nPrepared engine {:?}", existing_engine);
                        } else {
                            println!(
                                "\nNo TradeEngine with corresponding symbol {:?} and strategy {:?}",
                                new_trade.symbol, new_trade.strategy
                            );
                        }
                    }
                }
                Signal::OpenNewTrade(engine) => {
                    if engine.strategy == self.strategy_to_process {
                        // for existing_engine in trade_handlers.iter_mut() {
                        //     if existing_engine.symbol == engine.symbol
                        //         && existing_engine.can_accept_new_trade(&self.strategy_to_process)
                        //     {
                        //         println!("\n\nNew trade pushed {:?}", engine);

                        //         existing_engine.update_from(&engine).await;

                        //         let subscription = SubscriptionBuilder::new("abcde12345")
                        //             .mode(SubscriptionMode::Ltp)
                        //             .subscribe(
                        //                 SubscriptionExchange::NSEFO,
                        //                 vec![existing_engine.symbol_token.as_str()],
                        //             )
                        //             .build();

                        //         if let Err(e) = self
                        //             .tx_angelone_sender
                        //             .send(Signal::Subscribe(subscription))
                        //             .await
                        //         {
                        //             eprintln!("\nFailed to send subscription signal: {:?}", e);
                        //         }
                        //     } else {
                        //         println!(
                        //         "\nNo TradeEngine with corresponding symbol {:?} and strategy {:?}",
                        //         engine.symbol, engine.strategy
                        //     );
                        //     }
                        // }

                        // 1. Collect updated TradeEngine instances
                        let updated_trade_handlers: Vec<TradeEngine> = trade_handlers
                            .iter() // Iterate immutably
                            .map(|existing_engine| {
                                let mut updated_engine = existing_engine.clone(); // Clone for modification

                                if updated_engine.symbol == engine.symbol
                                    && updated_engine
                                        .can_accept_new_trade(&self.strategy_to_process)
                                {
                                    println!("\n\nNew trade pushed {:?}", engine);

                                    let engine_clone = engine.clone();
                                    let tx_angelone_sender_clone = self.tx_angelone_sender.clone();

                                    let mut updated_engine_clone = updated_engine.clone(); //clone the trade engine.

                                    let future = async move {
                                        updated_engine_clone.update_from(&engine_clone).await;

                                        let subscription = SubscriptionBuilder::new("abcde12345")
                                            .mode(SubscriptionMode::Ltp)
                                            .subscribe(
                                                SubscriptionExchange::NSEFO,
                                                vec![updated_engine_clone.symbol_token.as_str()],
                                            )
                                            .build();

                                        if let Err(e) = tx_angelone_sender_clone
                                            .send(Signal::Subscribe(subscription))
                                            .await
                                        {
                                            eprintln!(
                                                "\nFailed to send subscription signal: {:?}",
                                                e
                                            );
                                        }
                                    };
                                    tokio::spawn(future);
                                    updated_engine // Return the updated (or same) TradeEngine
                                } else {
                                    updated_engine // Return the unchanged TradeEngine
                                }
                            })
                            .collect();

                        // 2. Replace the original vector
                        trade_handlers = updated_trade_handlers;

                        // // 3. Print no match message if needed
                        // if !trade_handlers.iter().any(|existing_engine| {
                        //     existing_engine.symbol == engine.symbol
                        //         && existing_engine.can_accept_new_trade(&self.strategy_to_process)
                        // }) {
                        //     println!(
                        //         "\nNo TradeEngine with corresponding symbol {:?} and strategy {:?}",
                        //         engine.symbol, engine.strategy
                        //     );
                        // }
                    }
                }
                Signal::OrderPlaced(resp) => {
                    if (resp.strategy == self.strategy_to_process) {
                        if let Some(handler) = trade_handlers.iter_mut().find(|handler| {
                            handler.trade_status == TradeStatus::Confirming
                                && handler.trade_engine_id == resp.trade_engine_id
                                && handler.strategy == self.strategy_to_process // Check for matching trade_id
                        }) {
                            handler.trade_status = TradeStatus::Open;
                            handler.trade_entry_price = resp.price;
                            handler.update_values().await;

                            handler.trade_id = resp.trade_id;
                            handler.executed_trades += 1;
                            handler.prepare_exit_req();
                        }
                    }
                }
                Signal::OrderRejected(resp) => {
                    if (resp.strategy == self.strategy_to_process) {
                        if let Some(handler) = trade_handlers.iter_mut().find(|handler| {
                            handler.trade_status == TradeStatus::Confirming
                                && handler.trade_engine_id == resp.trade_engine_id
                                && handler.strategy == self.strategy_to_process // Check for matching trade_id
                        }) {
                            handler.trade_status = TradeStatus::Closed;
                            handler.exchange_type = ExchangeType::NFO;
                            handler.symbol_token = String::from("");
                            handler.trigger_price = 0.0;
                            handler.trading_symbol = String::from("");
                            handler.stop_loss_price - 0.0;

                            println!(
                                "\nOpen order Rejected ? Trade Engine Id -> {:?}",
                                handler.trade_engine_id
                            );
                        }
                    }
                }
                Signal::OrderError(resp) => {
                    if (resp.strategy == self.strategy_to_process) {
                        if let Some(handler) = trade_handlers.iter_mut().find(|handler| {
                            handler.trade_status == TradeStatus::Confirming
                                && handler.trade_engine_id == resp.trade_engine_id
                                && handler.strategy == self.strategy_to_process // Check for matching trade_id
                        }) {
                            handler.trade_status = TradeStatus::Closed;
                            handler.exchange_type = ExchangeType::NFO;
                            handler.symbol_token = String::from("");
                            handler.trigger_price = 0.0;
                            handler.trading_symbol = String::from("");
                            handler.stop_loss_price - 0.0;

                            println!(
                                "\nOpen order Rejected ? Trade Engine Id -> {:?}",
                                handler.trade_engine_id
                            );
                        }
                    }
                }
                Signal::CancelOrder {
                    symbol,
                    strategy,
                    transaction_type,
                    trigger_price,
                } => {
                    if (strategy == self.strategy_to_process) {
                        if let Some(handler) = trade_handlers.iter_mut().find(|handler| {
                            handler.trade_status == TradeStatus::Pending
                                && handler.symbol == symbol
                                && handler.strategy == strategy
                                && handler.position_type == transaction_type
                                && handler.trigger_price == trigger_price
                        }) {
                            handler.trade_status = TradeStatus::Closed;
                            handler.exchange_type = ExchangeType::NFO;
                            handler.symbol_token = String::from("");
                            handler.trigger_price = 0.0;
                            handler.trading_symbol = String::from("");
                            handler.stop_loss_price - 0.0;

                            println!(
                                "Open order cancelled ? Trade Engine Id -> {:?}",
                                handler.trade_engine_id
                            );
                        }
                    }
                }
                Signal::ForceSquareOff {
                    symbol,
                    strategy,
                    position_type,
                } => {
                    println!(
                        "Strategy {:?}, process strategy {:?}",
                        strategy, self.strategy_to_process
                    );
                    if (strategy == self.strategy_to_process) {
                        println!("\n\nhandlers {:?}", trade_handlers);
                        for handler in trade_handlers.iter_mut() {
                            if handler.trade_status == TradeStatus::Open
                                && handler.symbol == symbol
                                && handler.strategy == strategy
                                && handler.position_type == position_type
                            {
                                println!("Force {:?}", handler);
                                if let Some(_req) = &handler.exit_req {
                                    // found = true;
                                    handler.trade_status = TradeStatus::AwaitingConfirmation;
                                    handler
                                        .squareoff_trade(self.tx_order_processor.clone(), false)
                                        .await;

                                    let mut con = self
                                        .redis_client
                                        .get_multiplexed_async_connection()
                                        .await
                                        .unwrap();

                                    let _: () = con
                                        .set(
                                            format!(
                                                "squareoffReq:{}",
                                                handler.trade_id.to_string()
                                            ), // Convert u32 to String for key
                                            json!({
                                                "res":"acknowledge",
                                                "instance_id": "",
                                                "trade_id": handler.trade_id.to_string(), // Ensure `trade_id` is converted to String
                                                "client_id": handler.client_id.to_string(),
                                                "message":"Trade squared off successfully."
                                            })
                                            .to_string(), // Serialize JSON to String before storing in Redis
                                        )
                                        .await
                                        .unwrap();
                                }
                            }
                        }
                    }
                }
                Signal::SquareOffReject {
                    trade_id,
                    error,
                    message,
                } => {
                    if let Some(handler) = trade_handlers.iter_mut().find(|handler| {
                        handler.trade_status == TradeStatus::AwaitingConfirmation
                            && handler.trade_id == trade_id
                    }) {
                        handler.trade_status = TradeStatus::Open;
                        println!(
                            "Square Off Rejection -> error : {:?}, message : {:?}",
                            error, message
                        );
                    }
                }
                Signal::SquareOffError {
                    trade_id,
                    error,
                    message,
                } => {
                    if let Some(handler) = trade_handlers.iter_mut().find(|handler| {
                        handler.trade_status == TradeStatus::AwaitingConfirmation
                            && handler.trade_id == trade_id
                    }) {
                        handler.trade_status = TradeStatus::Open;
                        println!(
                            "Square Off Error -> error : {:?}, message : {:?}",
                            error, message
                        );
                    }
                }
                Signal::ClosePosition(trade_id) => {
                    if let Some(handler) = trade_handlers
                        .iter_mut()
                        .find(|handler| handler.trade_id == trade_id)
                    {
                        handler.reset().await;
                        println!("Trade Closed -> {:?}", handler.trade_engine_id);
                    }
                }
                Signal::NewTradeEngine(engine) => {
                    if self.strategy_to_process == engine.strategy {
                        println!(
                            "\nSymbol : {:?}, Engine = {:?}, Executed trades = {:?}, Status = {:?}, client id = {:?}",
                            engine.symbol,
                            engine.strategy,
                            engine.executed_trades,
                            engine.trade_status,
                            engine.client_id
                        );
                    }
                    if engine.strategy == self.strategy_to_process {
                        let mut new_engine = engine.clone(); //TradeEngine::create_trade_engine(engine).await.unwrap();

                        if !trade_handlers
                            .iter()
                            .any(|engine| engine.trade_engine_id == new_engine.trade_engine_id)
                        {
                            // println!(
                            //     "\nSymbol : {:?}, Engine = {:?}, Status = {:?}",
                            //     engine.symbol, engine.strategy, engine.trade_status
                            // );

                            if new_engine.trade_status == TradeStatus::Open {
                                println!("\nNew Trade handlers -> {:?}", new_engine);

                                new_engine.prepare_entry_req().await;
                                new_engine.prepare_exit_req().await;
                                new_engine.update_values().await;

                                let subscription = SubscriptionBuilder::new("abcde12345")
                                    .mode(SubscriptionMode::Ltp)
                                    .subscribe(
                                        SubscriptionExchange::NSEFO,
                                        vec![new_engine.symbol_token.as_str()],
                                    )
                                    .build();

                                if let Err(e) = self
                                    .tx_angelone_sender
                                    .send(Signal::Subscribe(subscription))
                                    .await
                                {
                                    eprintln!("\nFailed to send subscription signal: {:?}", e);
                                }
                            }

                            // println!("\nNew Engine trade Status = {:?}", new_engine.trade_status);

                            trade_handlers.push(new_engine);
                        } else {
                            println!(
                                "TradeEngine with trade_engine_id {:?} already exists.",
                                new_engine.trade_engine_id
                            );
                        }

                        // trade_handlers.push(new_engine);
                        // match add_trade_engine(config, tx_redis_clone.clone()).await {
                        //     Ok(trade_handler) => {
                        //         trade_handlers.push(trade_handler);
                        //     }
                        //     Err(e) => {
                        //         println!("\n\nError add trade handler... in main channel {:?}", e);
                        //     }
                        // }
                        // println!(
                        //     "\n\nTrade engines -> {:?}, Strategy -> {:?}",
                        //     trade_handlers.len(),
                        //     self.strategy_to_process
                        // );
                    }
                }
                Signal::AddClient {
                    api_key,
                    jwt_token,
                    client_id,
                } => {
                    // Here add a new client to place order to self.tx_order_processor
                    self.tx_order_processor
                        .send(Signal::AddClient {
                            api_key,
                            jwt_token,
                            client_id,
                        })
                        .await
                        .unwrap();
                }
                Signal::RemoveClient { client_id } => {
                    // Here add a new client to place order to self.tx_order_processor
                    self.tx_order_processor
                        .send(Signal::RemoveClient { client_id })
                        .await
                        .unwrap();
                }
                Signal::RemoveTradeEngine(trade_engine_id) => {
                    // Find and remove the trade engine with the matching ID from the vector
                    println!("Trying to remove trade engine -> {:?}", trade_engine_id);
                    trade_handlers.retain(|engine| engine.trade_engine_id != trade_engine_id);
                    println!(
                        "\n\nRemoved : No of trades engines = {:?}",
                        trade_handlers.len()
                    );
                }
                Signal::UpdateTradeEngine {
                    trade_engine_id,
                    client_id,
                    config,
                } => {
                    if let Some(handler) = trade_handlers.iter_mut().find(|handler| {
                        handler.client_id == client_id
                            && handler.trade_status == TradeStatus::Closed
                            && handler.trade_engine_id == trade_engine_id // Check for matching trade_id
                    }) {
                        handler.max_loss = config.max_loss;
                        handler.max_price = config.max_price;
                        handler.max_trades = config.max_trades;
                        handler.quantity = config.quantity;
                        handler.sl = config.sl;
                        handler.strategy = config.strategy;
                        handler.target = config.target;
                        handler.trailing_sl = config.trailing_sl;
                        handler.transaction_type = config.transaction_type;
                    }
                }
                Signal::RequestSquareOff {
                    client_id,
                    trade_id,
                    remove_trade_engine,
                } => {
                    // let mut found = false;

                    // for handler in trade_handlers.iter_mut() {
                    //     if handler.client_id == client_id
                    //         && handler.trade_status == TradeStatus::Open
                    //         && handler.trade_id == trade_id
                    //         && handler.strategy == self.strategy_to_process
                    //     // Check for matching trade_id
                    //     {
                    //         if let Some(_req) = &handler.exit_req {
                    //             // found = true;
                    //             handler.trade_status = TradeStatus::AwaitingConfirmation;
                    //             handler
                    //                 .squareoff_trade(
                    //                     self.tx_order_processor.clone(),
                    //                     remove_trade_engine,
                    //                 )
                    //                 .await;

                    //             let mut con = self
                    //                 .redis_client
                    //                 .get_multiplexed_async_connection()
                    //                 .await
                    //                 .unwrap();

                    //             let _: () = con
                    //                 .set(
                    //                     format!("squareoffReq:{}", trade_id.to_string()), // Convert u32 to String for key
                    //                     json!({
                    //                         "res":"acknowledge",
                    //                         "trade_id": trade_id.to_string(), // Ensure `trade_id` is converted to String
                    //                         "client_id": client_id.to_string(),
                    //                         "message":"Trade squared off successfully."
                    //                     })
                    //                     .to_string(), // Serialize JSON to String before storing in Redis
                    //                 )
                    //                 .await
                    //                 .unwrap();
                    //         }
                    //     }
                    // }

                    if let Some(handler) = trade_handlers.iter_mut().find(|found| {
                        found.client_id == client_id
                            && found.trade_status == TradeStatus::Open
                            && found.trade_id == trade_id // Corrected typo here
                            && found.strategy == self.strategy_to_process
                    }) {
                        if let Some(_req) = &handler.exit_req {
                            // found = true;
                            handler.trade_status = TradeStatus::AwaitingConfirmation;
                            handler
                                .squareoff_trade(
                                    self.tx_order_processor.clone(),
                                    remove_trade_engine,
                                )
                                .await;

                            // let mut con = self
                            //     .redis_client
                            //     .get_multiplexed_async_connection()
                            //     .await
                            //     .unwrap();

                            // let _: () = con
                            //     .set(
                            //         format!("squareoffReq:{}", trade_id.to_string()), // Convert u32 to String for key
                            //         json!({
                            //             "res":"acknowledge",
                            //             "trade_id": trade_id.to_string(), // Ensure `trade_id` is converted to String
                            //             "client_id": client_id.to_string(),
                            //             "message":"Trade squared off successfully."
                            //         })
                            //         .to_string(), // Serialize JSON to String before storing in Redis
                            //     )
                            //     .await
                            //     .unwrap();
                        }
                    }
                }
                Signal::SquaredOff {
                    trade_id,
                    strategy: _,
                    trade_engine_id
                } => {
                    // for handler in trade_handlers.iter_mut() {
                    //     if handler.client_id == client_id
                    //         && (handler.trade_status == TradeStatus::AwaitingConfirmation
                    //             || handler.trade_status == TradeStatus::Triggered)
                    //         && handler.trade_id == trade_id
                    //         && handler.strategy == self.strategy_to_process
                    //     {
                    //         handler.reset().await;
                    //     }
                    // }

                    if let Some(existing_engine) = trade_handlers.iter_mut().find(|found| {
                        found.client_id == client_id
                            && (found.trade_status == TradeStatus::AwaitingConfirmation
                                || found.trade_status == TradeStatus::Triggered)
                            && found.trade_id == trade_id
                            && found.strategy == self.strategy_to_process
                    }) {
                        existing_engine.reset().await;
                    }
                }
                Signal::Disconnect(client_id) => {
                    let mut _all_trade_closed = false;                    for handler in trade_handlers.iter_mut() {
                        // println!(
                        //     "\nHander client id = {:?}, trade status = {:?}",
                        //     handler.client_id, handler.trade_status
                        // );
                        if handler.client_id == client_id
                            && handler.trade_status == TradeStatus::Open
                        {
                            if let Some(_req) = &handler.exit_req {
                                handler
                                    .squareoff_trade(self.tx_order_processor.clone(), true)
                                    .await;
                            }
                        } else if handler.client_id == client_id
                            && handler.trade_status != TradeStatus::Open
                        {
                            handler.disconnect();
                        } else {
                            println!(
                                "Not open status client id -> {:?}, param client id -> {:?}, Status -> {:?}",
                                handler.client_id, client_id, handler.trade_status
                            );
                        }
                    }
                    // Retain only trade engines that do not match the `client_id`
                    // trade_handlers.retain(|engine| engine.client_id == client_id);

                    let mut i = 0;
                    while i < trade_handlers.len() {
                        if trade_handlers[i].client_id == client_id {
                            trade_handlers.remove(i); // Remove matching element
                        } else {
                            i += 1; // Only increment if not removed
                        }
                    }

                    self.tx_order_processor
                        .send(Signal::RemoveClient { client_id })
                        .await
                        .unwrap();

                    println!("\nHandlers count -> {:?}", trade_handlers.len());
                }

                Signal::TradeEngineDetails {
                    client_id,
                    strategy,
                    symbol,
                } => {
                    for handler in trade_handlers.iter_mut() {
                        if handler.client_id == client_id
                            && handler.strategy == strategy
                            && handler.symbol == symbol
                        {
                            println!("\n\n Details {:?}", handler);
                        }
                    }
                }
                Signal::Ping => {
                    // Do nothing
                }
                Signal::TestStream {
                    trade_engine_id,
                    client_id,
                    symbol,
                } => {
                    self.tx_redis
                        .send(Signal::TestStream {
                            trade_engine_id,
                            client_id,
                            symbol,
                        })
                        .await
                        .unwrap();
                }

                _ => {
                    println!("\n\nOther signal received inside.... {:?}", msg);
                }
            }
        }
    }

    async fn handle_buy_signal(&mut self, handler: &mut TradeEngine, ltp: f32) {
        // if (handler.client_id != 0) {
        println!(
            "\n\nclient id : {:?}, Trigger price : {:?}",
            handler.client_id, handler.trigger_price
        );
        // }
        if ltp > handler.trigger_price {
            handler.execute_trade(self.tx_order_processor.clone()).await;
        }
    }

    async fn handle_sell_signal(&mut self, handler: &mut TradeEngine, ltp: f32) {
        // if handler.transaction_type == TransactionType::SELL && ltp < handler.trigger_price {
        //     handler.execute_trade(tx_redis).await;
        // }
        if ltp < handler.trigger_price {
            handler.execute_trade(self.tx_order_processor.clone()).await;
        }
    }

    async fn open_new_trade(
        trade_handlers: &mut Vec<TradeEngine>,
        engine: &NewTrade,
        strategy_to_process: &Strategy,
        tx_angelone_sender: tokio::sync::mpsc::Sender<Signal>,
    ) -> Vec<Result<TradeEngine, String>> {
        let mut futures = Vec::new();
        for existing_engine in trade_handlers.iter() {
            if existing_engine.symbol == engine.symbol
                && existing_engine.can_accept_new_trade(strategy_to_process)
            {
                let engine_clone = engine.clone();
                let existing_engine_clone = existing_engine.clone();
                let tx_angelone_sender_clone = tx_angelone_sender.clone();

                let future = tokio::spawn(async move {
                    let mut updated_engine = existing_engine_clone.clone();
                    updated_engine.update_from(&engine_clone).await;

                    let subscription = SubscriptionBuilder::new("abcde12345")
                        .mode(SubscriptionMode::Ltp)
                        .subscribe(
                            SubscriptionExchange::NSEFO,
                            vec![updated_engine.symbol_token.as_str()],
                        )
                        .build();

                    if let Err(e) = tx_angelone_sender_clone
                        .send(Signal::Subscribe(subscription))
                        .await
                    {
                        eprintln!("\nFailed to send subscription signal: {:?}", e);
                        return Err(format!("Failed to send subscription signal: {:?}", e));
                    }
                    Ok(updated_engine)
                });
                futures.push(future);
            }
        }

        let mut results = Vec::new();
        for future in futures {
            match future.await {
                Ok(result) => results.push(result),
                Err(e) => results.push(Err(format!("Task panicked: {:?}", e))),
            }
        }

        results
    }
}

// async fn handle_buy_signal(handler: &mut TradeEngine, ltp: f32, tx_order: Sender<Signal>) {
//     if ltp > handler.trigger_price {
//         handler.execute_trade(tx_order).await;
//     }
// }

// async fn handle_sell_signal(handler: &mut TradeEngine, ltp: f32, tx_order: Sender<Signal>) {
//     // if handler.transaction_type == TransactionType::SELL && ltp < handler.trigger_price {
//     //     handler.execute_trade(tx_redis).await;
//     // }
//     if ltp < handler.trigger_price {
//         handler.execute_trade(tx_order).await;
//     }
// }

// PROCESS ALL TRADE HANDLERS BEGIN

// pub async fn process_engine(
//     strategy_to_process: Strategy,
//     rx_strategy: &mut tokio::sync::broadcast::Receiver<Signal>,
//     _tx_broadcast: Sender<Signal>,
//     _tx_main: Sender<Signal>,
//     tx_redis_clone: Sender<Signal>,
//     redis_client: redis::Client,
//     tx_angelone_sender: Sender<Signal>,
//     tx_order_processor: Sender<Signal>,
// ) {
//     let mut trade_handlers: Vec<TradeEngine> = Vec::new();

//     while let msg = rx_strategy.recv().await.unwrap() {
//         // Process the message
//         // println!("\n\nMessage is {:?}", msg);
//         match msg {
//             Signal::PriceFeed { token, ltp } => {
//                 // println!("\n {:?}, {:?}", token, ltp);
//                 let instrument = get_instrument(&token);
//                 for handler in trade_handlers.iter_mut() {
//                     match instrument {
//                         Instrument::NIFTY | Instrument::BANKNIFTY | Instrument::FINNIFTY => {
//                             if handler.trade_status == TradeStatus::Pending
//                                 && handler.symbol == instrument.to_string()
//                                 && handler.strategy == strategy_to_process
//                             {
//                                 match handler.position_type {
//                                     TransactionType::BUY => {
//                                         // handle_buy_signal(handler, ltp, tx_redis_clone.clone())
//                                         //     .await
//                                         handle_buy_signal(handler, ltp, tx_order_processor.clone())
//                                             .await
//                                     }
//                                     TransactionType::SELL => {
//                                         // handle_sell_signal(handler, ltp, tx_redis_clone.clone())
//                                         //     .await
//                                         handle_sell_signal(handler, ltp, tx_order_processor.clone())
//                                             .await
//                                     }
//                                     _ => {}
//                                 }
//                             }
//                         }
//                         Instrument::Other => {
//                             if handler.symbol_token == token
//                                 && handler.trade_status == TradeStatus::Open
//                             {
//                                 if handler.transaction_type == TransactionType::BUY {
//                                     println!(
//                                         "Buy Pnl : {:?},Symbol : {:?}, Entry : {:?}, Ltp : {:?}",
//                                         (ltp - handler.trade_entry_price) * handler.quantity as f32,
//                                         handler.trading_symbol,
//                                         handler.trade_entry_price,
//                                         ltp,
//                                     );

//                                     if ltp <= handler.stop_loss_price || ltp >= handler.target_price
//                                     // (handler.trade_entry_price - handler.sl)
//                                     {
//                                         handler.trade_status = TradeStatus::Triggered;
//                                         // handler
//                                         //     .squareoff_trade(&tx_redis_clone.clone(), false)
//                                         //     .await;
//                                         handler
//                                             .squareoff_trade(tx_order_processor.clone(), false)
//                                             .await;
//                                         println!("\n\nBuy square off trade");
//                                     }
//                                 } else {
//                                     println!(
//                                         "Sell Pnl : {:?}, Target price = {:?}, Sl price = {:?}",
//                                         (handler.trade_entry_price - ltp) * handler.quantity as f32,
//                                         handler.target_price,
//                                         handler.stop_loss_price
//                                     );

//                                     if ltp >= handler.stop_loss_price || ltp <= handler.target_price
//                                     // (handler.sl - handler.trade_entry_price)
//                                     {
//                                         handler.trade_status = TradeStatus::Triggered;
//                                         handler
//                                             .squareoff_trade(tx_order_processor.clone(), false)
//                                             .await;
//                                         println!("\n\nSell square off trade");
//                                     }
//                                 }
//                             }
//                         }
//                     }
//                 }
//             }
//             Signal::InitializeOpenTrade {
//                 new_trade,
//                 trade_res,
//             } => {
//                 if (new_trade.strategy == strategy_to_process) {
//                     if let Some(existing_engine) = trade_handlers.iter_mut().find(|existing| {
//                         existing.trade_engine_id == trade_res.trade_engine_id
//                             && existing.can_accept_new_trade(&strategy_to_process)
//                     }) {
//                         println!("\nNew trade initialized {:?}, {:?}", new_trade, trade_res);

//                         existing_engine.update_from(&new_trade).await;

//                         let subscription = SubscriptionBuilder::new("abcde12345")
//                             .mode(SubscriptionMode::Ltp)
//                             .subscribe(
//                                 SubscriptionExchange::NSEFO,
//                                 vec![existing_engine.symbol_token.as_str()],
//                             )
//                             .build();

//                         if (existing_engine.trade_engine_id == trade_res.trade_engine_id) {
//                             existing_engine.trade_entry_price = trade_res.price;
//                             existing_engine.update_values().await;

//                             existing_engine.trade_id = trade_res.trade_id;
//                             existing_engine.prepare_exit_req();
//                         }

//                         if let Err(e) = tx_angelone_sender
//                             .send(Signal::Subscribe(subscription))
//                             .await
//                         {
//                             eprintln!("\nFailed to send subscription signal: {:?}", e);
//                         }

//                         println!("\nPrepared engine {:?}", existing_engine);
//                     } else {
//                         println!(
//                             "\nNo TradeEngine with corresponding symbol {:?} and strategy {:?}",
//                             new_trade.symbol, new_trade.strategy
//                         );
//                     }
//                 }
//             }
//             Signal::OpenNewTrade(engine) => {
//                 // println!("\n Strategy = {:?}", strategy_to_process);
//                 if (engine.strategy == strategy_to_process) {
//                     if let Some(existing_engine) = trade_handlers.iter_mut().find(|existing| {
//                         existing.symbol == engine.symbol
//                             && existing.can_accept_new_trade(&strategy_to_process)
//                     }) {
//                         println!("\nNew trade pushed {:?}", engine);

//                         existing_engine.update_from(&engine).await;

//                         let subscription = SubscriptionBuilder::new("abcde12345")
//                             .mode(SubscriptionMode::Ltp)
//                             .subscribe(
//                                 SubscriptionExchange::NSEFO,
//                                 vec![existing_engine.symbol_token.as_str()],
//                             )
//                             .build();

//                         if let Err(e) = tx_angelone_sender
//                             .send(Signal::Subscribe(subscription))
//                             .await
//                         {
//                             eprintln!("\nFailed to send subscription signal: {:?}", e);
//                         }
//                     } else {
//                         println!(
//                             "\nNo TradeEngine with corresponding symbol {:?} and strategy {:?}",
//                             engine.symbol, engine.strategy
//                         );
//                     }
//                 }
//             }
//             Signal::OrderPlaced(resp) => {
//                 if (resp.strategy == strategy_to_process) {
//                     if let Some(handler) = trade_handlers.iter_mut().find(|handler| {
//                         handler.trade_status == TradeStatus::Confirming
//                             && handler.trade_engine_id == resp.trade_engine_id
//                             && handler.strategy == strategy_to_process // Check for matching trade_id
//                     }) {
//                         handler.trade_status = TradeStatus::Open;
//                         handler.trade_entry_price = resp.price;
//                         handler.update_values().await;

//                         handler.trade_id = resp.trade_id;
//                         handler.executed_trades += 1;
//                         handler.prepare_exit_req();
//                     }
//                 }
//             }
//             Signal::OrderRejected(resp) => {
//                 if (resp.strategy == strategy_to_process) {
//                     if let Some(handler) = trade_handlers.iter_mut().find(|handler| {
//                         handler.trade_status == TradeStatus::Confirming
//                             && handler.trade_engine_id == resp.trade_engine_id
//                             && handler.strategy == strategy_to_process // Check for matching trade_id
//                     }) {
//                         handler.trade_status = TradeStatus::Closed;
//                         handler.exchange_type = ExchangeType::NFO;
//                         handler.symbol_token = String::from("");
//                         handler.trigger_price = 0.0;
//                         handler.trading_symbol = String::from("");
//                         handler.stop_loss_price - 0.0;

//                         println!(
//                             "\nOpen order Rejected ? Trade Engine Id -> {:?}",
//                             handler.trade_engine_id
//                         );
//                     }
//                 }
//             }
//             Signal::OrderError(resp) => {
//                 if (resp.strategy == strategy_to_process) {
//                     if let Some(handler) = trade_handlers.iter_mut().find(|handler| {
//                         handler.trade_status == TradeStatus::Confirming
//                             && handler.trade_engine_id == resp.trade_engine_id
//                             && handler.strategy == strategy_to_process // Check for matching trade_id
//                     }) {
//                         handler.trade_status = TradeStatus::Closed;
//                         handler.exchange_type = ExchangeType::NFO;
//                         handler.symbol_token = String::from("");
//                         handler.trigger_price = 0.0;
//                         handler.trading_symbol = String::from("");
//                         handler.stop_loss_price - 0.0;

//                         println!(
//                             "\nOpen order Rejected ? Trade Engine Id -> {:?}",
//                             handler.trade_engine_id
//                         );
//                     }
//                 }
//             }
//             Signal::CancelOrder {
//                 symbol,
//                 strategy,
//                 transaction_type,
//                 trigger_price,
//             } => {
//                 if (strategy == strategy_to_process) {
//                     if let Some(handler) = trade_handlers.iter_mut().find(|handler| {
//                         handler.trade_status == TradeStatus::Pending
//                             && handler.symbol == symbol
//                             && handler.strategy == strategy
//                             && handler.position_type == transaction_type
//                             && handler.trigger_price == trigger_price
//                     }) {
//                         handler.trade_status = TradeStatus::Closed;
//                         handler.exchange_type = ExchangeType::NFO;
//                         handler.symbol_token = String::from("");
//                         handler.trigger_price = 0.0;
//                         handler.trading_symbol = String::from("");
//                         handler.stop_loss_price - 0.0;

//                         println!(
//                             "Open order cancelled ? Trade Engine Id -> {:?}",
//                             handler.trade_engine_id
//                         );
//                     }
//                 }
//             }
//             Signal::ForceSquareOff {
//                 symbol,
//                 strategy,
//                 position_type,
//             } => {
//                 println!(
//                     "Strategy {:?}, process strategy {:?}",
//                     strategy, strategy_to_process
//                 );
//                 if (strategy == strategy_to_process) {
//                     println!("\n\nhandlers {:?}", trade_handlers);
//                     if let Some(handler) = trade_handlers.iter_mut().find(|handler| {
//                         handler.trade_status == TradeStatus::Open
//                             && handler.symbol == symbol
//                             && handler.strategy == strategy
//                             && handler.position_type == position_type
//                     }) {
//                         println!("Force {:?}", handler);
//                         if let Some(_req) = &handler.exit_req {
//                             // found = true;
//                             handler.trade_status = TradeStatus::AwaitingConfirmation;
//                             handler
//                                 .squareoff_trade(tx_order_processor.clone(), false)
//                                 .await;

//                             let mut con = redis_client
//                                 .get_multiplexed_async_connection()
//                                 .await
//                                 .unwrap();

//                             let _: () = con
//                                 .set(
//                                     format!("squareoffReq:{}", handler.trade_id.to_string()), // Convert u32 to String for key
//                                     json!({
//                                         "res":"acknowledge",
//                                         "instance_id": "",
//                                         "trade_id": handler.trade_id.to_string(), // Ensure `trade_id` is converted to String
//                                         "client_id": handler.client_id.to_string(),
//                                         "message":"Trade squared off successfully."
//                                     })
//                                     .to_string(), // Serialize JSON to String before storing in Redis
//                                 )
//                                 .await
//                                 .unwrap();
//                         }
//                     }
//                 }
//             }
//             Signal::SquareOffReject {
//                 trade_id,
//                 error,
//                 message,
//             } => {
//                 if let Some(handler) = trade_handlers.iter_mut().find(|handler| {
//                     handler.trade_status == TradeStatus::AwaitingConfirmation
//                         && handler.trade_id == trade_id
//                 }) {
//                     handler.trade_status = TradeStatus::Open;
//                     println!(
//                         "Square Off Rejection -> error : {:?}, message : {:?}",
//                         error, message
//                     );
//                 }
//             }
//             Signal::SquareOffError {
//                 trade_id,
//                 error,
//                 message,
//             } => {
//                 if let Some(handler) = trade_handlers.iter_mut().find(|handler| {
//                     handler.trade_status == TradeStatus::AwaitingConfirmation
//                         && handler.trade_id == trade_id
//                 }) {
//                     handler.trade_status = TradeStatus::Open;
//                     println!(
//                         "Square Off Error -> error : {:?}, message : {:?}",
//                         error, message
//                     );
//                 }
//             }
//             Signal::ClosePosition(trade_id) => {
//                 if let Some(handler) = trade_handlers
//                     .iter_mut()
//                     .find(|handler| handler.trade_id == trade_id)
//                 {
//                     handler.trade_status = TradeStatus::Closed;
//                     handler.exchange_type = ExchangeType::NFO;
//                     handler.symbol_token = String::from("");
//                     handler.trigger_price = 0.0;
//                     handler.trading_symbol = String::from("");
//                     handler.stop_loss_price - 0.0;

//                     println!("Trade Closed -> {:?}", handler.trade_engine_id);
//                 }
//             }
//             Signal::NewTradeEngine(engine) => {
//                 if strategy_to_process == engine.strategy {
//                     println!(
//                         "\nSymbol : {:?}, Engine = {:?}, Executed trades = {:?}, Status = {:?}",
//                         engine.symbol, engine.strategy, engine.executed_trades, engine.trade_status
//                     );
//                 }
//                 if engine.strategy == strategy_to_process {
//                     let mut new_engine = engine.clone(); //TradeEngine::create_trade_engine(engine).await.unwrap();

//                     if !trade_handlers
//                         .iter()
//                         .any(|engine| engine.trade_engine_id == new_engine.trade_engine_id)
//                     {
//                         // println!(
//                         //     "\nSymbol : {:?}, Engine = {:?}, Status = {:?}",
//                         //     engine.symbol, engine.strategy, engine.trade_status
//                         // );

//                         if new_engine.trade_status == TradeStatus::Open {
//                             println!("\nNew Trade handlers -> {:?}", new_engine);

//                             new_engine.prepare_entry_req().await;
//                             new_engine.prepare_exit_req().await;
//                             new_engine.update_values().await;

//                             let subscription = SubscriptionBuilder::new("abcde12345")
//                                 .mode(SubscriptionMode::Ltp)
//                                 .subscribe(
//                                     SubscriptionExchange::NSEFO,
//                                     vec![new_engine.symbol_token.as_str()],
//                                 )
//                                 .build();

//                             if let Err(e) = tx_angelone_sender
//                                 .send(Signal::Subscribe(subscription))
//                                 .await
//                             {
//                                 eprintln!("\nFailed to send subscription signal: {:?}", e);
//                             }
//                         }

//                         // println!("\nNew Engine trade Status = {:?}", new_engine.trade_status);

//                         trade_handlers.push(new_engine);
//                     } else {
//                         println!(
//                             "TradeEngine with trade_engine_id {:?} already exists.",
//                             new_engine.trade_engine_id
//                         );
//                     }

//                     // trade_handlers.push(new_engine);
//                     // match add_trade_engine(config, tx_redis_clone.clone()).await {
//                     //     Ok(trade_handler) => {
//                     //         trade_handlers.push(trade_handler);
//                     //     }
//                     //     Err(e) => {
//                     //         println!("\n\nError add trade handler... in main channel {:?}", e);
//                     //     }
//                     // }
//                     // println!(
//                     //     "\n\nTrade engines -> {:?}, Strategy -> {:?}",
//                     //     trade_handlers.len(),
//                     //     strategy_to_process
//                     // );
//                 }
//             }
//             Signal::AddClient {
//                 api_key,
//                 jwt_token,
//                 client_id,
//             } => {
//                 // Here add a new client to place order to tx_order_processor
//                 tx_order_processor
//                     .send(Signal::AddClient {
//                         api_key,
//                         jwt_token,
//                         client_id,
//                     })
//                     .await
//                     .unwrap();
//             }
//             Signal::RemoveClient { client_id } => {
//                 // Here add a new client to place order to tx_order_processor
//                 tx_order_processor
//                     .send(Signal::RemoveClient { client_id })
//                     .await
//                     .unwrap();
//             }
//             Signal::RemoveTradeEngine(trade_engine_id) => {
//                 // Find and remove the trade engine with the matching ID from the vector
//                 trade_handlers.retain(|engine| engine.trade_engine_id != trade_engine_id);
//                 println!(
//                     "\n\nRemoved : No of trades engines = {:?}",
//                     trade_handlers.len()
//                 );
//             }
//             Signal::UpdateTradeEngine {
//                 trade_engine_id,
//                 client_id,
//                 config,
//             } => {
//                 if let Some(handler) = trade_handlers.iter_mut().find(|handler| {
//                     handler.client_id == client_id
//                         && handler.trade_status == TradeStatus::Closed
//                         && handler.trade_engine_id == trade_engine_id // Check for matching trade_id
//                 }) {
//                     handler.max_loss = config.max_loss;
//                     handler.max_price = config.max_price;
//                     handler.max_trades = config.max_trades;
//                     handler.quantity = config.quantity;
//                     handler.sl = config.sl;
//                     handler.strategy = config.strategy;
//                     handler.target = config.target;
//                     handler.trailing_sl = config.trailing_sl;
//                     handler.transaction_type = config.transaction_type;
//                 }
//             }
//             Signal::SquareOffTradeReq {
//                 instance_id,
//                 client_id,
//                 trade_id,
//                 remove_trade_engine,
//             } => {
//                 // let mut found = false;

//                 println!("Instance id {:?}", instance_id);

//                 for handler in trade_handlers.iter_mut() {
//                     if handler.client_id == client_id
//                         && handler.trade_status == TradeStatus::Open
//                         && handler.trade_id == trade_id
//                         && handler.strategy == strategy_to_process
//                     // Check for matching trade_id
//                     {
//                         if let Some(_req) = &handler.exit_req {
//                             // found = true;
//                             handler.trade_status = TradeStatus::AwaitingConfirmation;
//                             handler
//                                 .squareoff_trade(tx_redis_clone.clone(), remove_trade_engine)
//                                 .await;

//                             let mut con = redis_client
//                                 .get_multiplexed_async_connection()
//                                 .await
//                                 .unwrap();

//                             let _: () = con
//                                 .set(
//                                     format!("squareoffReq:{}", trade_id.to_string()), // Convert u32 to String for key
//                                     json!({
//                                         "res":"acknowledge",
//                                         "instance_id": instance_id.as_str(),
//                                         "trade_id": trade_id.to_string(), // Ensure `trade_id` is converted to String
//                                         "client_id": client_id.to_string(),
//                                         "message":"Trade squared off successfully."
//                                     })
//                                     .to_string(), // Serialize JSON to String before storing in Redis
//                                 )
//                                 .await
//                                 .unwrap();
//                         }
//                     }
//                 }
//             }
//             Signal::Disconnect(client_id) => {
//                 for handler in trade_handlers.iter_mut() {
//                     println!(
//                         "\nHander client id = {:?}, trade status = {:?}",
//                         handler.client_id, handler.trade_status
//                     );
//                     if handler.client_id == client_id && handler.trade_status == TradeStatus::Open {
//                         if let Some(_req) = &handler.exit_req {
//                             handler
//                                 .squareoff_trade(tx_order_processor.clone(), true)
//                                 .await;
//                         }
//                     } else if handler.client_id == client_id
//                         && handler.trade_status != TradeStatus::Open
//                     {
//                         handler.disconnect();
//                         tx_order_processor
//                             .send(Signal::RemoveClient { client_id })
//                             .await
//                             .unwrap();
//                     } else {
//                         println!(
//                             "Handler client id -> {:?}, param client id -> {:?}, Status -> {:?}",
//                             handler.client_id, client_id, handler.trade_status
//                         );
//                     }
//                 }
//                 // Retain only trade engines that do not match the `client_id`
//                 // trade_handlers.retain(|engine| engine.client_id == client_id);

//                 let mut i = 0;
//                 while i < trade_handlers.len() {
//                     if trade_handlers[i].client_id == client_id {
//                         trade_handlers.remove(i); // Remove matching element
//                     } else {
//                         i += 1; // Only increment if not removed
//                     }
//                 }
//                 println!("\nHandlers count -> {:?}", trade_handlers.len());
//             }

//             Signal::TradeEngineDetails {
//                 client_id,
//                 strategy,
//                 symbol,
//             } => {
//                 for handler in trade_handlers.iter_mut() {
//                     if handler.client_id == client_id
//                         && handler.strategy == strategy
//                         && handler.symbol == symbol
//                     {
//                         println!("\n\n Details {:?}", handler);
//                     }
//                 }
//             }
//             Signal::TestStream {
//                 trade_engine_id,
//                 client_id,
//                 symbol,
//             } => {
//                 tx_redis_clone
//                     .send(Signal::TestStream {
//                         trade_engine_id,
//                         client_id,
//                         symbol,
//                     })
//                     .await
//                     .unwrap();
//             }

//             _ => {
//                 println!("\n\nOther signal received inside.... {:?}", msg);
//             }
//         }
//     }
// }

// PROCESS TRADE HANDLERS END
