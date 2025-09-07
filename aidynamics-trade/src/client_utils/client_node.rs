use crate::order_processor::order_processor::OrderProcessor;
use crate::redis_utils::Signal;
use crate::trade_engine::TradeEngine;
use crate::trade_engine::TradeStatus;
// use crate::types::ExchangeType;
use crate::types::TransactionType;
use crate::websocket::angel_one_websocket::{
    SubscriptionBuilder, SubscriptionExchange, SubscriptionMode,
};
use chrono::Utc;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use tokio::sync::mpsc::Sender;
use tracing::{error, info, warn};

use crate::SmartConnect;

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

/// Represents a single client's container that manages multiple TradeEngines.
#[derive(Debug, Clone)]
pub struct ClientNode {
    /// Unique identifier for the client.
    pub client_id: u32,

    /// HashMap holding all TradeEngines for this client.
    pub trade_engines: HashMap<u32, TradeEngine>,

    /// Sender for broadcasting signals to multiple receivers.
    // pub tx_broadcast: Arc<tokio::sync::broadcast::Sender<Signal>>,

    /// Sender for sending signals to the main application thread.
    pub tx_main: Sender<Signal>,

    /// Sender for sending signals to the Redis service.
    pub tx_redis: Sender<Signal>,

    /// Sender for sending signals to the AngelOne service.
    pub tx_angelone_sender: Sender<Signal>,

    /// Sender for sending signals to the order processing component.
    // pub tx_order_processor: Arc<Sender<Signal>>,

    /// Contains only handler trade_engine_id which status is not Closed
    pub active_trade_ids: HashSet<u32>,

    /// The strategy that this processor is responsible for.
    // pub strategy_to_process: Strategy,

    /// Contains only handler trade_engine_id which status is Closed
    pub handler_ids: HashSet<u32>,

    /// Angelone Client to place orders
    pub angelone_client: Option<Arc<SmartConnect>>,
}

impl ClientNode {
    /// Creates a new `ClientNode` with the provided `client_id` and `trade_engines`.
    ///
    /// # Example
    /// ```
    /// let engines = HashMap::new();
    /// let client_node = ClientNode::new(1, engines);
    /// ```
    pub fn new(
        client_id: u32,
        trade_engines: HashMap<u32, TradeEngine>,
        // tx_broadcast: Arc<tokio::sync::broadcast::Sender<Signal>>,
        tx_main: Sender<Signal>,
        tx_redis: Sender<Signal>,
        // tx_order_processor: Arc<Sender<Signal>>,
        active_trade_ids: HashSet<u32>,
        // strategy_to_process: Strategy,
        handler_ids: HashSet<u32>,
        tx_angelone_sender: Sender<Signal>,
    ) -> Self {
        Self {
            client_id,
            trade_engines,
            // tx_broadcast,
            tx_main,
            tx_redis,
            active_trade_ids,
            // tx_order_processor,
            // strategy_to_process,
            handler_ids,
            tx_angelone_sender,
            angelone_client: None,
        }
    }

    /// Handles all operations of engine according to strategy in seperate task
    /// Signals are received here excluding PriceFeed
    pub async fn process_engine(
        &mut self,
        // mut rx_message: tokio::sync::broadcast::Receiver<Signal>,
        mut rx_message: tokio::sync::mpsc::Receiver<Signal>,
    ) {
        let tx_main_cln = Arc::new(self.tx_main.clone()); // Wrap in Arc once outside the loop
        println!(
            "\nClient Node started for client id = {:?} with total trade engines = {:?}\n",
            self.client_id,
            self.trade_engines.len()
        );
        let mut price_hashmap: HashMap<String, f32> = HashMap::new();
        loop {
            let tx_main_clone = tx_main_cln.clone();
            // while let message = self.rx_message.recv().await {
            // Process the message
            let message = rx_message.recv().await;
            match message {
                // Ok(msg) => {
                Some(msg) => {
                    // println!("Broadcast = {:?}", msg);
                    match msg {
                        Signal::PriceFeed { token, ltp } => {
                            let instrument = get_instrument(&token);
                            let active_ids = self.active_trade_ids.clone();

                            for trade_engine_id in active_ids {
                                // if let Some(handler) = self.trade_engines.get_mut(&trade_engine_id) {
                                {
                                    if let Some(handler) =
                                        self.trade_engines.get_mut(&trade_engine_id)
                                    {
                                        match instrument {
                                            Instrument::NIFTY
                                            | Instrument::BANKNIFTY
                                            | Instrument::FINNIFTY
                                            | Instrument::SENSEX => {
                                                if handler.trade_status == TradeStatus::Pending
                                                    // && handler.strategy == self.strategy_to_process
                                                    && handler.symbol == instrument.to_string()
                                                {
                                                    match handler.position_type {
                                                        TransactionType::BUY => {
                                                            // handler.trade_status =
                                                            //     TradeStatus::Processing; // lock it
                                                            if ltp > handler.trigger_price {
                                                                handler
                                                                    .execute_trade(Arc::new(
                                                                        self.tx_main.clone(),
                                                                    ))
                                                                    .await;
                                                            }
                                                        }
                                                        TransactionType::SELL => {
                                                            // handler.trade_status =
                                                            //     TradeStatus::Processing; // lock it
                                                            if ltp < handler.trigger_price {
                                                                handler
                                                                    .execute_trade(Arc::new(
                                                                        self.tx_main.clone(),
                                                                    ))
                                                                    .await;
                                                            }
                                                        }
                                                        _ => {}
                                                    }
                                                }
                                            }
                                            Instrument::Other => {
                                                price_hashmap.insert(token.clone(), ltp.clone());
                                                // println!(
                                                //     "\nPrice hashmap : token = {:?},  ltp = {:?}",
                                                //     token, ltp
                                                // );
                                                if handler.symbol_token == token
                                                    && handler.trade_status == TradeStatus::Open
                                                {
                                                    if handler.transaction_type
                                                        == TransactionType::BUY
                                                    {
                                                        // info!(
                                                        //     ?handler.transaction_type,
                                                        //     pnl = (ltp - handler.trade_entry_price)
                                                        //         * handler.quantity as f32,
                                                        //     Symbol = handler.trading_symbol,
                                                        //     entry = handler.trade_entry_price,
                                                        //     Ltp = ltp
                                                        // );

                                                        handler.trail_stop_loss(ltp);

                                                        if ltp <= handler.stop_loss_price
                                                            || ltp >= handler.target_price
                                                            || handler.exceeds_threshold()
                                                        // (handler.trade_entry_price - handler.sl)
                                                        {
                                                            handler.trade_status =
                                                                TradeStatus::Triggered;

                                                            handler
                                                                .squareoff_trade(
                                                                    Arc::new(self.tx_main.clone()),
                                                                    false,
                                                                )
                                                                .await;
                                                            println!("\n");
                                                            info!("\n\nBuy square off trade");
                                                        }
                                                    } else {
                                                        // info!(
                                                        //     ?handler.transaction_type, pnl = (handler.trade_entry_price - ltp)
                                                        //     * handler.quantity as f32, target_price = handler.target_price,
                                                        //     sl_price = handler.stop_loss_price);
                                                        // handler.trail_stop_loss(ltp);
                                                        println!("\n");
                                                        info!("Entry Price {},SL price = {}, Target price = {}, LTP = {}",handler.trade_entry_price, handler.stop_loss_price, handler.target_price, ltp);
                                                        handler.trail_stop_loss(ltp);

                                                        if ltp >= handler.stop_loss_price
                                                            || ltp <= handler.target_price
                                                            || handler.exceeds_threshold()
                                                        // (handler.sl - handler.trade_entry_price)
                                                        {
                                                            handler.trade_status =
                                                                TradeStatus::Triggered;
                                                            handler
                                                                .squareoff_trade(
                                                                    Arc::new(self.tx_main.clone()),
                                                                    false,
                                                                )
                                                                .await;
                                                            println!("\n");
                                                            info!("sell_squared off");
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        // Signal::InitializeOpenTrade {
                        //     new_trade,
                        //     trade_res,
                        // } => {
                        //     // if new_trade.strategy == self.strategy_to_process {
                        //     if let Some(existing_engine) =
                        //         self.trade_engines.get_mut(&trade_res.trade_engine_id)
                        //     {
                        //         let can_accept = existing_engine.can_accept_new_trade(&new_trade.strategy)
                        //         println!(
                        //             "\nCan accept new trade ? = {:?} for engine id = {:?}",
                        //             can_accept, existing_engine.trade_engine_id
                        //         );
                        //         if can_accept {
                        //             info!("new_trade_init = {:?} Res {:?}", new_trade, trade_res);

                        //             existing_engine.update_from(&new_trade).await;

                        //             let subscription = SubscriptionBuilder::new("abcde12345")
                        //                 .mode(SubscriptionMode::Ltp)
                        //                 .subscribe(
                        //                     SubscriptionExchange::NSEFO,
                        //                     vec![existing_engine.symbol_token.as_str()],
                        //                 )
                        //                 .build();

                        //             existing_engine.trade_entry_price = trade_res.price;
                        //             existing_engine.update_values().await;

                        //             existing_engine.trade_id = trade_res.trade_id;
                        //             existing_engine.prepare_exit_req().await;

                        //             self.active_trade_ids.insert(trade_res.trade_engine_id);

                        //             if let Err(e) = self
                        //                 .tx_angelone_sender
                        //                 .send(Signal::Subscribe(subscription))
                        //                 .await
                        //             {
                        //                 error!("send_subscription_failed {:?}", e);
                        //             }

                        //             // println!("\nPrepared engine {:?}", existing_engine);
                        //         }
                        //         // else {
                        //         //     println!(
                        //         //     "\nNo TradeEngine with corresponding symbol {:?} and strategy {:?}",
                        //         //     new_trade.symbol, new_trade.strategy
                        //         // );
                        //         // }
                        //     }
                        //     // }
                        // }
                        Signal::OpenNewTrade(engine) => {
                            println!(
                                "\nOpen new trade Client Id = {:?} : Strategy = {:?}",
                                self.client_id, engine.strategy
                            );
                            // if engine.strategy == self.strategy_to_process {
                            let engine_clone = engine.clone();
                            // let tx_angelone_sender_clone = self.tx_angelone_sender.clone();
                            // let mut trade_handlers_clone = self.trade_engines.clone(); // Clone trade_handlers
                            // let tx_main_new = tx_main_clone.clone();

                            // tokio::spawn(async move {
                            for (_, existing_engine) in self.trade_engines.iter_mut() {
                                let accepted: bool =
                                    existing_engine.can_accept_new_trade(&engine_clone.strategy);
                                if existing_engine.symbol == engine_clone.symbol && accepted {
                                    println!("\n");
                                    info!("\nTrade_pushed {:?}", engine_clone);

                                    existing_engine.update_from(&engine_clone).await;

                                    // Update active_trade_ids (need a way to communicate back if needed)
                                    // active_trade_ids_clone.insert(*id); // Cannot modify outside the spawned task

                                    let _ = tx_main_clone
                                        .send(Signal::UpdateActiveTrades(existing_engine.clone()))
                                        .await;

                                    let subscription = SubscriptionBuilder::new("abcde12345")
                                        .mode(SubscriptionMode::Ltp)
                                        .subscribe(
                                            SubscriptionExchange::NSEFO,
                                            vec![existing_engine.symbol_token.as_str()],
                                        )
                                        .build();

                                    if let Err(e) = self
                                        .tx_angelone_sender
                                        .send(Signal::Subscribe(subscription))
                                        .await
                                    {
                                        error!("\nFailed to send subscription signal: {:?}", e);
                                    }
                                }
                            }
                            // });
                        }
                        Signal::UpdateActiveTrades(trade_engine) => {
                            // if trade_engine.strategy == self.strategy_to_process {
                            if trade_engine.trade_status == TradeStatus::Closed {
                                self.active_trade_ids.remove(&trade_engine.trade_engine_id);
                                self.handler_ids.insert(trade_engine.trade_engine_id);
                            } else {
                                self.active_trade_ids.insert(trade_engine.trade_engine_id);
                                self.handler_ids.remove(&trade_engine.trade_engine_id);
                            }
                            // self.trade_engines
                            //     .insert(trade_engine.trade_engine_id, trade_engine);
                            // }

                            let new_engine = trade_engine.clone();
                            let mut to_update = vec![];

                            for (_, existing_engine) in self.trade_engines.iter_mut() {
                                if existing_engine.strategy == new_engine.strategy
                                    && existing_engine.symbol == new_engine.symbol
                                    && existing_engine.client_id == new_engine.client_id
                                {
                                    to_update.push(existing_engine.trade_engine_id);
                                }
                            }

                            // Apply updates after iteration
                            for id in to_update {
                                self.trade_engines.insert(id, new_engine.clone());
                            }

                            println!(
                                "\nUpdated trade engines Client id = {:?} : Engines = {:?}\n",
                                self.client_id,
                                self.trade_engines.len()
                            );
                        }

                        // Signal::UpdateMargin { client_id, status } => {
                        //     if client_id != 0 {
                        //         for id in self.active_trade_ids.clone() {
                        //             if let Some(handler) = self.trade_engines.get_mut(&id) {
                        //                 if handler.client_id == client_id {
                        //                     // handler.margin = status;
                        //                     info!("Margin received {:?}", status)
                        //                 }
                        //             }
                        //         }
                        //         for id in self.handler_ids.clone() {
                        //             if let Some(handler) = self.trade_engines.get_mut(&id) {
                        //                 if handler.client_id == client_id {
                        //                     handler.margin = status
                        //                 }
                        //             }
                        //         }
                        //     }
                        // }
                        Signal::OrderPlaced(resp) => {
                            // if resp.strategy == self.strategy_to_process {
                            if let Some(handler) = self.trade_engines.get_mut(&resp.trade_engine_id)
                            {
                                if handler.trade_status == TradeStatus::Confirming
                                    || handler.trade_status == TradeStatus::Executing
                                {
                                    handler.trade_status = TradeStatus::Open;
                                    handler.trade_entry_price = resp.price;
                                    handler.update_values().await;
                                    handler.trade_id = resp.trade_id;
                                    handler.executed_trades += 1;
                                    handler.prepare_exit_req().await;
                                    handler.execution_time = Utc::now().timestamp();
                                    self.active_trade_ids.insert(resp.trade_engine_id);
                                    if handler.client_id != 0 {
                                        self.tx_redis
                                            .send(Signal::UpdateUniqueOrderId {
                                                position_type: 1,
                                                trade_id: handler.trade_id,
                                                order_id: resp.order_id.to_string(),
                                                unique_order_id: handler
                                                    .unique_order_id
                                                    .clone()
                                                    .unwrap(),
                                            })
                                            .await
                                            .unwrap();
                                    }
                                }
                            }
                            // }
                        }
                        Signal::OrderRejected(resp) => {
                            // if resp.strategy == self.strategy_to_process {
                            if let Some(handler) = self.trade_engines.get_mut(&resp.trade_engine_id)
                            {
                                if handler.trade_status == TradeStatus::Confirming
                                    || handler.trade_status == TradeStatus::Executing
                                {
                                    // handler.trade_status = TradeStatus::Closed;
                                    // handler.exchange_type = ExchangeType::NFO;
                                    // handler.symbol_token = String::from("");
                                    // handler.trigger_price = 0.0;
                                    // handler.trading_symbol = String::from("");
                                    // handler.stop_loss_price - 0.0;

                                    // println!(
                                    //     "\nOpen order Rejected ? Trade Engine Id -> {:?}",
                                    //     handler.trade_engine_id
                                    // );
                                    self.active_trade_ids.remove(&resp.trade_engine_id);
                                    handler.reset().await;
                                }
                            }
                            // }
                        }
                        Signal::OrderError(resp) => {
                            // if resp.strategy == self.strategy_to_process {
                            if let Some(handler) = self.trade_engines.get_mut(&resp.trade_engine_id)
                            {
                                if handler.trade_status == TradeStatus::Confirming
                                    || handler.trade_status == TradeStatus::Executing
                                {
                                    // handler.trade_status = TradeStatus::Closed;
                                    // handler.exchange_type = ExchangeType::NFO;
                                    // handler.symbol_token = String::from("");
                                    // handler.trigger_price = 0.0;
                                    // handler.trading_symbol = String::from("");
                                    // handler.stop_loss_price - 0.0;

                                    // println!(
                                    //     "\nOpen order Rejected ? Trade Engine Id -> {:?}",
                                    //     handler.trade_engine_id
                                    // );
                                    self.active_trade_ids.remove(&resp.trade_engine_id);
                                    handler.reset().await;
                                }
                            }
                            // }
                        }
                        Signal::CancelOrder {
                            symbol,
                            strategy,
                            transaction_type,
                            trigger_price,
                        } => {
                            // if strategy == self.strategy_to_process {
                            // let mut trade_handlers_clone = self.trade_engines.clone(); // Clone trade_handlers
                            //                                                            // let tx_broadcast = self.tx_broadcast.clone();
                            // let tx_main_new_clone_2 = tx_main_clone.clone();
                            // let active_trade_ids = self.active_trade_ids.clone();

                            // tokio::spawn(async move {
                            for id in self.active_trade_ids.clone() {
                                if let Some(handler) = self.trade_engines.get_mut(&id) {
                                    if (handler.trade_status == TradeStatus::Pending
                                        || handler.trade_status == TradeStatus::Confirming
                                        || handler.trade_status == TradeStatus::Executing)
                                        && handler.symbol == symbol
                                        && handler.strategy == strategy
                                        && handler.position_type == transaction_type
                                        && handler.trigger_price == trigger_price
                                    {
                                        // handler.trade_status = TradeStatus::Closed;
                                        // handler.exchange_type =
                                        //     if handler.exchange_type == ExchangeType::NFO {
                                        //         ExchangeType::NFO
                                        //     } else {
                                        //         ExchangeType::BFO
                                        //     };
                                        // handler.symbol_token = String::from("");
                                        // handler.trigger_price = 0.0;
                                        // handler.trading_symbol = String::from("");
                                        // let _ = handler.stop_loss_price - 0.0;
                                        handler.reset().await;
                                        let _ = tx_main_clone
                                            .send(Signal::UpdateActiveTrades(handler.clone()))
                                            .await;
                                        println!("\n");
                                        info!(
                                            "Order cancelled ? Client id -> {:?} Engine Id -> {:?}",
                                            handler.client_id, handler.trade_engine_id
                                        );
                                    }
                                }
                            }
                            // });
                            // }
                        }
                        Signal::UpdateTradeStatus {
                            trade_engine_id,
                            status,
                        } => {
                            if let Some(engine) = self.trade_engines.get_mut(&trade_engine_id) {
                                // if engine.strategy == self.strategy_to_process {
                                engine.trade_status = status.clone();
                                if status == TradeStatus::Closed {
                                    self.active_trade_ids.remove(&engine.trade_engine_id);
                                    self.handler_ids.insert(engine.trade_engine_id);
                                }
                                // }
                            }
                        }
                        Signal::ForceSquareOff {
                            symbol,
                            strategy,
                            position_type,
                        } => {
                            // // if strategy == self.strategy_to_process {
                            // let tx_order_main = self.tx_main.clone();
                            // // let tx_broadcast = self.tx_broadcast.clone();
                            // let active_trade_ids = self.active_trade_ids.clone();

                            // let mut trade_handlers_map: HashMap<u32, TradeEngine> = HashMap::new();
                            // for id in self.active_trade_ids.clone() {
                            //     if let Some(handler) = self.trade_engines.get(&id) {
                            //         trade_handlers_map.insert(id, handler.clone());
                            //     }
                            // }
                            println!("\n");
                            info!(
                                "\nSquared Off Active handlers Client id = {:?} : {:?}",
                                self.client_id,
                                self.active_trade_ids.len()
                            );

                            for (_, handler) in self.trade_engines.iter_mut() {
                                if handler.trade_status == TradeStatus::Open
                                    && handler.symbol == symbol
                                    && handler.strategy == strategy
                                    && handler.position_type == position_type
                                {
                                    println!("\n");
                                    info!("\nForce {:?}", handler);
                                    if let Some(_req) = &handler.exit_req {
                                        // found = true;
                                        // let tx_main_new = tx_main_clone.clone();
                                        // let _ = tx_main_new
                                        //     .send(Signal::UpdateTradeStatus {
                                        //         trade_engine_id: handler.trade_engine_id,
                                        //         status: TradeStatus::AwaitingConfirmation,
                                        //     })
                                        //     .await;
                                        handler.trade_status = TradeStatus::AwaitingConfirmation;
                                        handler
                                            .squareoff_trade(Arc::new(self.tx_main.clone()), false)
                                            .await;
                                    }
                                }
                            }
                        }
                        Signal::SquareOffReject {
                            trade_id: _,
                            error,
                            message,
                            trade_engine_id,
                        } => {
                            if let Some(handler) = self.trade_engines.get_mut(&trade_engine_id) {
                                if handler.trade_status == TradeStatus::AwaitingConfirmation
                                    || handler.trade_status == TradeStatus::SquaringOff
                                    || handler.trade_status == TradeStatus::Triggered
                                {
                                    // && handler.strategy == self.strategy_to_process

                                    handler.trade_status = TradeStatus::Open;
                                    error!(
                                        "Square Off Rejection -> error : {:?}, message : {:?}",
                                        error, message
                                    );
                                }
                            }
                        }
                        Signal::SquareOffError {
                            trade_id: _,
                            error,
                            message,
                            trade_engine_id,
                        } => {
                            if let Some(handler) = self.trade_engines.get_mut(&trade_engine_id) {
                                if handler.trade_status == TradeStatus::AwaitingConfirmation
                                    || handler.trade_status == TradeStatus::SquaringOff
                                    || handler.trade_status == TradeStatus::Triggered
                                // && handler.strategy == self.strategy_to_process
                                {
                                    handler.trade_status = TradeStatus::Open;
                                    println!("\n");
                                    info!(
                                        "Square Off Error Client id -> {:?} -> error : {:?}, message : {:?}",
                                        handler.client_id,error, message
                                    );
                                }
                            }
                        }
                        Signal::ClosePosition(trade_engine_id) => {
                            if let Some(handler) = self.trade_engines.get_mut(&trade_engine_id) {
                                {
                                    self.active_trade_ids.remove(&trade_engine_id);
                                    handler.reset().await;
                                    println!("\n");
                                    info!(
                                        "Trade Closed Client id -> {:?} -> {:?}",
                                        handler.client_id, handler.trade_engine_id
                                    );
                                }
                            }
                        }
                        Signal::SetUniqueOrderId(trade_engine_id, order_id) => {
                            for (_, handler) in self.trade_engines.iter_mut() {
                                if handler.trade_engine_id == trade_engine_id {
                                    handler.unique_order_id = Some(order_id.clone());
                                }
                            }
                        }
                        Signal::NewTradeEngine(engine) => {
                            let mut code_should_run = true;
                            if let Some(handler) = self
                                .trade_engines
                                .values()
                                .find(|h| h.client_id != engine.client_id)
                            {
                                code_should_run = false;
                                println!("\n");
                                warn!(
                                    "Found another client node client id = {:?}",
                                    handler.client_id
                                );
                            }

                            if code_should_run {
                                let mut new_engine = engine.clone(); //TradeEngine::create_trade_engine(engine).await.unwrap();

                                if self.trade_engines.contains_key(&new_engine.trade_engine_id) {
                                    warn!(
                                    "Trade engine with ID {} already exists. Skipping addition.",
                                    new_engine.trade_engine_id
                                );
                                // Skip adding this trade engine
                                // }
                                // if let Some(_) = self.trade_engines.get_mut(&engine.trade_engine_id) {
                                // } else {
                                } else {
                                    println!(
                                        "\nNew trade engine Client id = {:?} : Engine Id {:?}",
                                        new_engine.client_id, new_engine.trade_engine_id
                                    );
                                    println!("\n");
                                    info!(
                                        engine_id = ?engine.trade_engine_id,
                                        symbol = ?engine.symbol,
                                        strategy = ?engine.strategy,
                                        executed = ?engine.executed_trades,
                                        status = ?engine.trade_status,
                                        client_id = engine.client_id
                                    );
                                    if new_engine.trade_status == TradeStatus::Open {
                                        println!("\n");
                                        info!(new_engine = ?new_engine);

                                        new_engine.prepare_entry_req().await;
                                        new_engine.prepare_exit_req().await;
                                        new_engine.update_values().await;

                                        self.active_trade_ids.insert(new_engine.trade_engine_id);

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
                                            error!(subscription_failed = ?e);
                                        }
                                    } else {
                                        self.handler_ids.insert(new_engine.trade_engine_id);
                                    }

                                    self.trade_engines
                                        .insert(new_engine.trade_engine_id, new_engine);
                                }

                                // }
                            }
                        }
                        Signal::AddClient {
                            api_key,
                            jwt_token,
                            client_id: _,
                        } => {
                            if self.angelone_client.is_none() {
                                let new_client =
                                    SmartConnect::new_with_jwt(api_key, Some(&jwt_token))
                                        .await
                                        .unwrap();
                                self.angelone_client = Some(Arc::new(new_client));
                            }
                        }
                        Signal::ExecuteOrder {
                            order_req,
                            strategy,
                            trade_engine_id,
                            client_id,
                        } => {
                            if client_id == self.client_id {
                                let order_clone = order_req.clone();
                                // let tx_brd = tx_broadcast.clone();
                                let tx_main_clone = tx_main_clone.clone();
                                let strategy_clone = strategy.clone(); // Clone strategy outside of the task so it is not moved.
                                let tx_redis_clone = self.tx_redis.clone();

                                println!("\n");
                                info!(
                                    "Executing -> Engine = {:?} client id = {:?}",
                                    trade_engine_id,
                                    client_id.clone()
                                );

                                if let Some(client_arc) = self.angelone_client.clone() {
                                    let client_arc_clone = client_arc.clone();
                                    let price_hash_clone = price_hashmap.clone();

                                    println!("\n Engines size : {:?}", self.trade_engines.len());
                                    if let Some(engine) =
                                        self.trade_engines.get_mut(&trade_engine_id)
                                    {
                                        if engine.trade_status == TradeStatus::Confirming {
                                            engine.trade_status = TradeStatus::Executing;

                                            tokio::spawn(async move {
                                                OrderProcessor::handle_order_placement(
                                                    client_id,
                                                    client_arc_clone,
                                                    order_clone,
                                                    tx_main_clone,
                                                    trade_engine_id,
                                                    strategy_clone,
                                                    tx_redis_clone,
                                                    price_hash_clone,
                                                )
                                                .await;
                                            });
                                        }
                                    }
                                } else {
                                    // error!()
                                    error!(
                                        "Client executor not found for client id: {:?}",
                                        client_id
                                    );
                                }
                            }
                        }
                        Signal::SquareOffTrade {
                            client_id,
                            order_req,
                            trade_id,
                            trade_engine_id,
                            remove_trade_engine,
                            strategy,
                        } => {
                            let tx_redis_clone = self.tx_redis.clone();

                            println!("\nClient id in Square off => {:?}", client_id);

                            if let Some(client_arc) = self.angelone_client.clone() {
                                let client_arc_clone = client_arc.clone();
                                let price_hashmap_clone = price_hashmap.clone();
                                if let Some(engine) = self.trade_engines.get_mut(&trade_engine_id) {
                                    if engine.trade_status == TradeStatus::Triggered
                                        || engine.trade_status == TradeStatus::AwaitingConfirmation
                                    {
                                        engine.trade_status = TradeStatus::SquaringOff;
                                        tokio::spawn(async move {
                                            OrderProcessor::handle_squareoff_placement(
                                                client_id,
                                                client_arc_clone,
                                                order_req,
                                                tx_main_clone.clone(),
                                                trade_engine_id,
                                                trade_id,
                                                strategy,
                                                remove_trade_engine,
                                                tx_redis_clone,
                                                price_hashmap_clone,
                                            )
                                            .await;
                                        });
                                    }
                                }
                            } else {
                                eprintln!(
                                    "Client not found for client id: {:?} Square Off",
                                    client_id
                                );
                            }
                        }
                        Signal::RemoveClient { client_id } => {
                            // Here add a new client to place order to self.tx_order_processor
                            if self.client_id == client_id && self.angelone_client.is_some() {
                                self.angelone_client = None;
                                // self.tx_main
                                //     .send(Signal::RemoveClient { client_id })
                                //     .await
                                //     .unwrap();
                            }
                        }
                        Signal::RemoveTradeEngine(trade_engine_id) => {
                            // Find and remove the trade engine with the matching ID from the vector
                            println!("\n");
                            info!(remove_trade_engine = ?trade_engine_id);
                            self.trade_engines.remove(&trade_engine_id);
                            self.active_trade_ids.remove(&trade_engine_id);
                            self.handler_ids.remove(&trade_engine_id);
                            if self.trade_engines.is_empty() {
                                // This will exit the loop and stop the tokio::spawn task
                                println!("\nDESCTROYING CLIENT NODE : {:?}", self.client_id);
                                break;
                            }
                        }
                        Signal::UpdateTradeEngine {
                            trade_engine_id,
                            client_id,
                            config,
                        } => {
                            if let Some(handler) = self.trade_engines.get_mut(&trade_engine_id) {
                                if handler.client_id == client_id
                                    && handler.trade_status == TradeStatus::Closed
                                // && handler.strategy == self.strategy_to_process
                                {
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
                        }
                        Signal::RequestSquareOff {
                            client_id,
                            trade_id,
                            remove_trade_engine,
                            trade_engine_id,
                        } => {
                            if let Some(handler) = self.trade_engines.get_mut(&trade_engine_id) {
                                if handler.client_id == client_id
                                    && handler.trade_status == TradeStatus::Open
                                    && handler.trade_id == trade_id
                                // Corrected typo here
                                // && handler.strategy == self.strategy_to_process
                                {
                                    if let Some(_req) = &handler.exit_req {
                                        // found = true;
                                        handler.trade_status = TradeStatus::AwaitingConfirmation;
                                        handler
                                            .squareoff_trade(
                                                Arc::new(self.tx_main.clone()),
                                                remove_trade_engine,
                                            )
                                            .await;
                                    }
                                }
                            }
                        }
                        Signal::SquaredOff {
                            trade_id,
                            strategy: _,
                            trade_engine_id,
                            order_id,
                        } => {
                            if let Some(t_eng_id) = self.active_trade_ids.get(&trade_engine_id) {
                                if let Some(handler) = self.trade_engines.get_mut(t_eng_id) {
                                    if (handler.trade_status == TradeStatus::AwaitingConfirmation
                                        || handler.trade_status == TradeStatus::Triggered
                                        || handler.trade_status == TradeStatus::SquaringOff)
                                        && handler.trade_id == trade_id
                                    // && handler.strategy == self.strategy_to_process
                                    {
                                        if handler.client_id != 0 {
                                            self.tx_redis
                                                .send(Signal::UpdateUniqueOrderId {
                                                    position_type: 0,
                                                    trade_id: handler.trade_id,
                                                    order_id: order_id,
                                                    unique_order_id: handler
                                                        .unique_order_id
                                                        .clone()
                                                        .unwrap(),
                                                })
                                                .await
                                                .unwrap();
                                        }

                                        tx_main_clone
                                            .send(Signal::DeleteActiveTradeIds(*t_eng_id))
                                            .await
                                            .unwrap();
                                        let remove_now = handler.remove_trade_engine.clone();
                                        if remove_now {
                                            handler.reset().await;
                                            let _ = tx_main_clone
                                                .send(Signal::RemoveTradeEngine(*t_eng_id))
                                                .await;
                                        } else {
                                            handler.reset().await;
                                        }
                                    }
                                }
                            }
                        }
                        Signal::DeleteActiveTradeIds(trade_engine_id) => {
                            self.active_trade_ids.remove(&trade_engine_id);
                            self.handler_ids.insert(trade_engine_id);
                        }
                        Signal::Disconnect(client_id) => {
                            // let mut trade_handlers_clone = self.trade_engines.clone();
                            let tx_order_main = Arc::new(self.tx_main.clone());
                            // let active_trade_ids = self.active_trade_ids.clone();
                            // // let tx_broadcast = self.tx_broadcast.clone();
                            let handler_ids = self.handler_ids.clone();
                            // tokio::spawn(async move {
                            for id in self.active_trade_ids.clone() {
                                if let Some(handler) = self.trade_engines.get_mut(&id) {
                                    if handler.client_id == client_id
                                        && handler.trade_status == TradeStatus::Open
                                    {
                                        if let Some(_req) = &handler.exit_req {
                                            handler
                                                .squareoff_trade(tx_order_main.clone(), true)
                                                .await;
                                            handler.disconnect().await;
                                        }
                                    }
                                }
                            }
                            for id in handler_ids {
                                if let Some(handler) = self.trade_engines.get_mut(&id) {
                                    if handler.client_id == client_id && client_id != 0 {
                                        tx_main_clone
                                            .send(Signal::RemoveTradeEngine(
                                                handler.trade_engine_id,
                                            ))
                                            .await
                                            .unwrap();
                                    }
                                }
                            }
                            tx_order_main
                                .send(Signal::RemoveClient { client_id })
                                .await
                                .unwrap();

                            // println!("\nHandlers count -> {:?}", trade_handlers_clone.len());
                            // });
                        }

                        Signal::TradeEngineDetails {
                            client_id,
                            strategy,
                            symbol,
                        } => {
                            for (_, handler) in self.trade_engines.iter_mut() {
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
                        _ => {}
                    }
                }
                None => {
                    // println!("No message");
                } // Err(e) => {
                  //     println!("Error {:?}", e);
                  // }
            }
        }
    }
}
