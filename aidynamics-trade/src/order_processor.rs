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
        funds::MarginCalculatorRes,
        market::{LtpDataReq, LtpDataRes, MarketDataReq},
        order::{self, PlaceOrderReq},
        portfolio::Position,
        redis_utils::Signal,
        server_utils::ServerHttp,
        trade_engine::TradeRes,
        types::ExchangeType,
    };

    use super::*;

    impl OrderProcessor {
        /// Associated function to place order to open new position
        pub async fn handle_order_placement(
            client_id: u32,
            angelone_client: Arc<SmartConnect>,
            order_req: PlaceOrderReq,
            tx_main: Arc<tokio::sync::mpsc::Sender<Signal>>,
            trade_engine_id: u32,
            strategy: Strategy,
            tx_redis: Sender<Signal>,
            // price_hasmap: HashMap<String, f32>,
        ) {
            let order_clone = order_req.clone();
            println!("\nClient id found {:?}, strategy {:?}", client_id, strategy);
            if client_id == 0 {
                // let mut ltp: f64 = 0.0;

                // match price_hasmap.get(&order_clone.inner.symbol_token) {
                //     Some(price) => {
                //         ltp = *price as f64;
                //     }
                //     None => {
                //         tracing::error!("Error fetching LTP data from hashmap");
                //     }
                // }

                // // let ltp = client_arc_clone.ltp_data(&ltp_data_req).await.unwrap();
                // let ltp_price = ltp as f32;

                let mut ltp: f64 = 0.0;

                let ltp_data_req = LtpDataReq {
                    exchange: order_req.inner.exchange,
                    trading_symbol: order_req.clone().inner.trading_symbol,
                    symbol_token: order_req.clone().inner.symbol_token,
                };
                match angelone_client.ltp_data(&ltp_data_req).await {
                    Ok(ltp_data) => {
                        ltp = ltp_data.ltp;
                    }
                    Err(e) => {
                        tracing::error!("Error fetching LTP data: {:?}", e);
                        // tx_redis.send(Signal::Error(e)).await.unwrap();
                        // return;
                    }
                }

                let quantity_result = usize::from_str(&order_clone.inner.quantity);

                let position = SmartConnect::new_margin_calculator_position(
                    order_clone.inner.exchange,
                    ProductType::IntraDay,
                    order_clone.transaction_type,
                    &order_clone.inner.symbol_token,
                    ltp as f32,
                    quantity_result.unwrap(),
                );

                let margin = match angelone_client.calculate_margin(&[position]).await {
                    Ok(mar) => Some(mar),
                    Err(e) => {
                        tracing::error!("Error while calculating margin {:?}", e);
                        None
                    }
                };

                let margin_required = if let Some(mar) = margin {
                    mar.total_margin_required
                } else {
                    0.0
                };

                tx_redis
                    .send(Signal::ExecuteDemoTrade {
                        client_id,
                        trade_engine_id,
                        order_req: order_req.clone(),
                        price: ltp as f32,
                        margin_required: margin_required as f32,
                    })
                    .await
                    .unwrap();
                return;
            }

            // tracing::info!("\nOrder : {:?} client id : {:?}", &order_clone, &client_id);

            match angelone_client.place_order(&order_clone).await {
                Ok(response) => {
                    println!("\n");
                    tracing::info!("Order res => {:?}", response);
                    println!("\n\n");
                    if let Some(is) = response.unique_order_id {
                        tx_main
                            .send(Signal::SetUniqueOrderId(trade_engine_id, is.clone()))
                            .await
                            .unwrap();
                    }
                }
                Err(e) => {
                    tracing::error!("Error placing order: {:?}", e);
                    tx_main
                        .send(Signal::OrderError(TradeRes {
                            trade_engine_id,
                            trade_id: 0,
                            price: 0.0,
                            strategy: strategy,
                            order_id: "".to_string(),
                        }))
                        .await
                        .unwrap();
                    
                }
            };
        }

        /// Associated function to place order to square off position
        pub async fn handle_squareoff_placement(
            client_id: u32,
            angelone_client: Arc<SmartConnect>,
            order_req: PlaceOrderReq,
            tx_main: Arc<Sender<Signal>>,
            trade_engine_id: u32,
            trade_id: u32,
            strategy: Strategy,
            remove_trade_engine: bool,
            tx_redis: Sender<Signal>,
            // price_hasmap: HashMap<String, f32>,
        ) {
            if remove_trade_engine {
                tx_main
                    .send(Signal::SetRemoveTradeEngine(trade_engine_id))
                    .await
                    .unwrap();
            }
            let order_clone = order_req.clone();
            // info!("Order == {:?}", order_clone);
            if client_id == 0 {
                let mut ltp: f64 = 0.0;

                let ltp_data_req = LtpDataReq {
                    exchange: order_clone.inner.exchange,
                    trading_symbol: order_clone.inner.trading_symbol,
                    symbol_token: order_clone.inner.symbol_token,
                };
                // let ltp = angelone_client.ltp_data(&ltp_data_req).await.unwrap();

                match angelone_client.ltp_data(&ltp_data_req).await {
                    Ok(ltp_data) => {
                        ltp = ltp_data.ltp;
                    }
                    Err(e) => {
                        tracing::error!("Error fetching LTP data: {:?}", e);
                        // tx_redis.send(Signal::Error(e)).await.unwrap();
                        // return;
                    }
                }

                let order_clone = order_req.clone();
                tx_redis
                    .send(Signal::UpdateSquareOffDemo {
                        client_id,
                        trade_id,
                        remove_trade_engine,
                        strategy: strategy,
                        order: order_clone,
                        price: ltp as f32,
                    })
                    .await
                    .unwrap();
                return;
            }

            // let res = angelone_client.place_order(&order_req).await.unwrap();

            match angelone_client.place_order(&order_clone).await {
                Ok(response) => {
                    tracing::info!("Order res => {:?}", response);

                    if let Some(order_id) = response.unique_order_id {
                        tx_main
                            .send(Signal::SetUniqueOrderId(trade_engine_id, order_id.clone()))
                            .await
                            .unwrap();
                    }
                }
                Err(e) => {
                    tracing::error!("Error placing square off order {:?}", e);
                }
            }

            // -------------------------------------------------
        }
    }
}
