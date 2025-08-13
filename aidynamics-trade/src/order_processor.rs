#![allow(unused_imports)]
use crate::{
    market::{SearchScrip, SearchScripRes},
    order::{IndividualOrderStatus, OrderSetter, PlaceOrderReq, PlaceOrderRes},
    trade_engine::{Segment, Strategy, TradeEngine},
    types::{MarketDataExchange, ProductType, TransactionType},
    Result, SmartConnect,
};
use aidynamics_trade_utils::Error;
use tracing::{info, instrument};

use futures::future::join_all;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::mpsc::Sender;
use tokio::task;

// Assuming SmartConnect, PlaceOrderReq, and PlaceOrderRes are defined elsewhere

/// Order process module for basically placing orders concurrently
pub mod order_processor {

    /// Struct to create module where Order functions
    /// can be called from any where in project
    pub struct OrderProcessor;

    use std::{process::exit, thread::sleep, time::Duration};

    use rand::Error;
    use tokio::signal;

    use crate::{
        market::{LtpDataReq, MarketDataReq},
        order::{self, PlaceOrderReq},
        redis_utils::Signal,
        server_utils::ServerHttp,
        trade_engine::TradeRes,
        // trade_handler::Payload,
        types::ExchangeType,
    };

    use super::*;

    impl OrderProcessor {
        /// Associated function to place order for new position
        pub async fn handle_order_placement(
            client_id: u32,
            client_arc_clone: Arc<SmartConnect>,
            order_req: PlaceOrderReq,
            tx_main_clone: Arc<tokio::sync::mpsc::Sender<Signal>>,
            trade_engine_id: u32,
            strategy: Strategy,
            tx_redis: Sender<Signal>,
        ) {
            let order_clone = order_req.clone();
            println!("Client id found ${:?}", client_id);
            if client_id == 0 {
                let ltp_data_req = LtpDataReq {
                    exchange: order_clone.inner.exchange,
                    trading_symbol: order_clone.clone().inner.trading_symbol,
                    symbol_token: order_clone.inner.symbol_token.clone(),
                };
                let ltp = client_arc_clone.ltp_data(&ltp_data_req).await.unwrap();

                let quantity_result = usize::from_str(&order_clone.inner.quantity);

                let position = SmartConnect::new_margin_calculator_position(
                    order_clone.inner.exchange,
                    ProductType::IntraDay,
                    order_clone.transaction_type,
                    &order_clone.inner.symbol_token,
                    ltp.ltp,
                    quantity_result.unwrap(),
                );

                let margin = client_arc_clone
                    .calculate_margin(&[position])
                    .await
                    .unwrap();
                let margin_required = margin.total_margin_required;

                tx_redis
                    .send(Signal::ExecuteDemoTrade {
                        client_id,
                        trade_engine_id,
                        order_req: order_req.clone(),
                        price: ltp.ltp as f32,
                        margin_required: margin_required as f32,
                    })
                    .await
                    .unwrap();
                return;
            }

            tracing::info!("\nOrder : {:?} client id : {:?}", &order_clone, &client_id);

            let res = client_arc_clone.place_order(&order_clone).await.unwrap();
            tracing::info!("Order res => {:?}", res);

            if let Some(is) = res.unique_order_id {
                let ind_status = client_arc_clone.order_status(is).await.unwrap();

                tracing::info!("Waiting for 2 seconds.");

                sleep(Duration::from_millis(2000));

                tracing::info!("Individual status: {:?}", ind_status);

                let position = SmartConnect::new_margin_calculator_position(
                    ind_status.order.exchange,
                    ProductType::IntraDay,
                    ind_status.order.transaction_type,
                    &ind_status.order.symbol_token,
                    ind_status.order.price,
                    usize::from_str(ind_status.order.quantity.as_str()).unwrap(),
                );

                let margin = client_arc_clone
                    .calculate_margin(&[position])
                    .await
                    .unwrap();
                let margin_required = margin.total_margin_required;

                tracing::info!("margin required {:?}", margin_required);

                if ind_status.order.status == "rejected"
                    || ind_status.order.order_status == "rejected"
                {
                    tx_main_clone
                        .send(Signal::OrderRejected(TradeRes {
                            trade_engine_id,
                            trade_id: 0,
                            price: 0.0,
                            strategy: strategy.clone(),
                        }))
                        .await
                        .unwrap();
                } else if ind_status.order.status == "error"
                    || ind_status.order.order_status == "error"
                {
                    tx_main_clone
                        .send(Signal::OrderError(TradeRes {
                            trade_engine_id,
                            trade_id: 0,
                            price: 0.0,
                            strategy: strategy.clone(),
                        }))
                        .await
                        .unwrap();
                }

                let order_clone = ind_status.clone();

                let mut price: f32 = 0.00;
                if order_clone.order.average_price != 0.00 {
                    price = order_clone.order.average_price as f32;
                } else {
                    price = order_clone.order.price as f32;
                }

                let redis_data = Signal::UpdateExecutionData {
                    client_id,
                    trade_engine_id,
                    order: order_clone.clone(),
                    price: price as f32,
                    margin_required: margin_required as f32,
                    strategy,
                };
                tx_redis.send(redis_data).await.unwrap();
            }
        }

        /// Associated function to place order to square off open position

        pub async fn handle_squareoff_placement(
            client_id: u32,
            client_arc_clone: Arc<SmartConnect>,
            order_req: PlaceOrderReq,
            tx_main_clone: Arc<Sender<Signal>>,
            trade_engine_id: u32,
            trade_id: u32,
            strategy: Strategy,
            remove_trade_engine: bool,
            tx_redis: Sender<Signal>,
        ) {
            // Commented for production
            //let order_clone = order_req.clone();
            // ----------------------------
            let order_clone = order_req.clone();
            info!("Order == {:?}", order_clone);
            if client_id == 0 {
                let ltp_data_req = LtpDataReq {
                    exchange: order_clone.inner.exchange,
                    trading_symbol: order_clone.clone().inner.trading_symbol,
                    symbol_token: order_clone.inner.symbol_token.clone(),
                };
                let ltp = client_arc_clone.ltp_data(&ltp_data_req).await.unwrap();

                tx_redis
                    .send(Signal::UpdateSquareOffDemo {
                        client_id,
                        trade_id,
                        remove_trade_engine,
                        strategy: strategy.clone(),
                        order: order_clone,
                        price: ltp.ltp as f32,
                    })
                    .await
                    .unwrap();
                return;
            }

            let res = client_arc_clone.place_order(&order_req).await.unwrap();

            if let Some(order_id) = res.unique_order_id {
                let ind_status = client_arc_clone.order_status(order_id).await.unwrap();
                let order = ind_status.clone();
                let order_clone = order.clone();

                let price = order.order.average_price;

                tx_redis
                    .send(Signal::UpdateSquareOff {
                        client_id,
                        trade_id,
                        remove_trade_engine,
                        trade_engine_id,
                        strategy: strategy.clone(),
                        price: price as f32,
                        order,
                    })
                    .await
                    .unwrap();

                info!("Waiting for 2 seconds.");
                sleep(Duration::from_millis(2000));

                info!("\nIndividual status: {:?}", ind_status);
                // let status = order_clone.order.status.clone();
                // let order_status = order_clone.order.order_status.clone();

                if order_clone.order.status == "rejected"
                    || order_clone.order.order_status == "rejected"
                {
                    let _ = tx_main_clone
                        .send(Signal::OrderRejected(TradeRes {
                            trade_engine_id,
                            trade_id: 0,
                            price: 0.0,
                            strategy: strategy.clone(),
                        }))
                        .await;
                } else if order_clone.order.status == "error"
                    || order_clone.order.order_status == "error"
                {
                    let _ = tx_main_clone
                        .send(Signal::OrderError(TradeRes {
                            trade_engine_id,
                            trade_id: 0,
                            price: 0.0,
                            strategy: strategy.clone(),
                        }))
                        .await;
                }

                if remove_trade_engine {
                    let _ = tx_main_clone
                        .send(Signal::RemoveTradeEngine(trade_engine_id))
                        .await;
                }
            }
            // -------------------------------------------------
        }
    }
}
