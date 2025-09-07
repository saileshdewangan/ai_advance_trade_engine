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
        /// Associated function to place order for new position
        // pub async fn handle_order_placement(
        //     client_id: u32,
        //     client_arc_clone: Arc<SmartConnect>,
        //     order_req: PlaceOrderReq,
        //     tx_main_clone: Arc<tokio::sync::mpsc::Sender<Signal>>,
        //     trade_engine_id: u32,
        //     strategy: Strategy,
        //     tx_redis: Sender<Signal>,
        //     price_hasmap: HashMap<String, f32>,
        // ) {
        //     let order_clone = order_req.clone();
        //     println!("\nClient id found {:?}, strategy {:?}", client_id, strategy);
        //     if client_id == 0 {
        //         let mut ltp: f64 = 0.0;

        //         match price_hasmap.get(&order_clone.inner.symbol_token) {
        //             Some(price) => {
        //                 ltp = *price as f64;
        //             }
        //             None => {
        //                 tracing::error!("Error fetching LTP data from hashmap");
        //             }
        //         }

        //         // let ltp = client_arc_clone.ltp_data(&ltp_data_req).await.unwrap();
        //         let ltp_price = ltp as f32;

        //         let quantity_result = usize::from_str(&order_clone.inner.quantity);

        //         let position = SmartConnect::new_margin_calculator_position(
        //             order_clone.inner.exchange,
        //             ProductType::IntraDay,
        //             order_clone.transaction_type,
        //             &order_clone.inner.symbol_token,
        //             ltp_price,
        //             quantity_result.unwrap(),
        //         );

        //         let margin = match client_arc_clone.calculate_margin(&[position]).await {
        //             Ok(mar) => Some(mar),
        //             Err(e) => {
        //                 tracing::error!("Error while calculating margin {:?}", e);
        //                 None
        //             }
        //         };

        //         let margin_required = if let Some(mar) = margin {
        //             mar.total_margin_required
        //         } else {
        //             0.0
        //         };

        //         tx_redis
        //             .send(Signal::ExecuteDemoTrade {
        //                 client_id,
        //                 trade_engine_id,
        //                 order_req: order_req.clone(),
        //                 price: ltp_price as f32,
        //                 margin_required: margin_required as f32,
        //             })
        //             .await
        //             .unwrap();
        //         return;
        //     }

        //     // tracing::info!("\nOrder : {:?} client id : {:?}", &order_clone, &client_id);

        //     match client_arc_clone.place_order(&order_clone).await {
        //         Ok(response) => {
        //             println!("\n");
        //             tracing::info!("Order res => {:?}", response);
        //             println!("\n\n");
        //             if let Some(is) = response.unique_order_id {
        //                 tx_main_clone
        //                     .send(Signal::SetUniqueOrderId(trade_engine_id, is.clone()))
        //                     .await
        //                     .unwrap();

        //                 let mut ind_status: Option<IndividualOrderStatus> = None;
        //                 let mut check_count = 0;
        //                 loop {
        //                     let is_clone = is.clone();
        //                     println!("\n");
        //                     tracing::info!("Waiting for 2 seconds.");
        //                     sleep(Duration::from_millis(2000));
        //                     ind_status = match client_arc_clone.order_status(is_clone).await {
        //                         Ok(status) => Some(status),
        //                         Err(e) => {
        //                             tracing::error!("Error fetching order status: {:?}", e);
        //                             None
        //                         }
        //                     };
        //                     if let Some(ref status) = ind_status {
        //                         if status.order.status != "open"
        //                             && status.order.order_status != "open"
        //                         {
        //                             break;
        //                         }
        //                     }
        //                     if check_count == 3 {
        //                         tracing::error!("Order status check timed out after 6 seconds.");
        //                         break;
        //                     } else {
        //                         check_count += 1;
        //                     }
        //                 }

        //                 println!("\n\n");
        //                 tracing::info!("Individual status: {:?}", ind_status);
        //                 println!("\n\n");
        //                 let individual_status = ind_status.unwrap();
        //                 let position = SmartConnect::new_margin_calculator_position(
        //                     individual_status.order.exchange,
        //                     ProductType::IntraDay,
        //                     individual_status.order.transaction_type,
        //                     &individual_status.order.symbol_token,
        //                     individual_status.order.average_price,
        //                     usize::from_str(individual_status.order.quantity.as_str()).unwrap(),
        //                 );

        //                 let margin = match client_arc_clone.calculate_margin(&[position]).await {
        //                     Ok(mar) => Some(mar),
        //                     Err(e) => {
        //                         tracing::error!("Error while calculating margin {:?}", e);
        //                         None
        //                     }
        //                 };
        //                 let margin_required = if let Some(val) = margin {
        //                     val.total_margin_required
        //                 } else {
        //                     0.0
        //                 };

        //                 // tracing::info!("margin required {:?}", margin_required);

        //                 if individual_status.order.status == "rejected"
        //                     || individual_status.order.order_status == "rejected"
        //                 {
        //                     tx_main_clone
        //                         .send(Signal::OrderRejected(TradeRes {
        //                             trade_engine_id,
        //                             trade_id: 0,
        //                             price: 0.0,
        //                             strategy: strategy.clone(),
        //                         }))
        //                         .await
        //                         .unwrap();
        //                 } else if individual_status.order.status == "error"
        //                     || individual_status.order.order_status == "error"
        //                 {
        //                     tx_main_clone
        //                         .send(Signal::OrderError(TradeRes {
        //                             trade_engine_id,
        //                             trade_id: 0,
        //                             price: 0.0,
        //                             strategy: strategy.clone(),
        //                         }))
        //                         .await
        //                         .unwrap();
        //                 }

        //                 let order_clone = individual_status.clone();

        //                 let price: f32 = order_clone.order.average_price as f32;

        //                 let redis_data = Signal::UpdateExecutionData {
        //                     client_id,
        //                     trade_engine_id,
        //                     order: order_clone.clone(),
        //                     price: price as f32,
        //                     margin_required: margin_required as f32,
        //                     strategy,
        //                 };
        //                 tx_redis.send(redis_data).await.unwrap();
        //             }
        //         }
        //         Err(e) => {
        //             tracing::error!("Error placing order: {:?}", e);
        //         }
        //     };
        // }

        pub async fn handle_order_placement(
            client_id: u32,
            client_arc_clone: Arc<SmartConnect>,
            order_req: PlaceOrderReq,
            tx_main_clone: Arc<tokio::sync::mpsc::Sender<Signal>>,
            trade_engine_id: u32,
            strategy: Strategy,
            tx_redis: Sender<Signal>,
            price_hasmap: HashMap<String, f32>,
        ) {
            let order_clone = order_req.clone();
            println!("\nClient id found {:?}, strategy {:?}", client_id, strategy);
            if client_id == 0 {
                let mut ltp: f64 = 0.0;

                match price_hasmap.get(&order_clone.inner.symbol_token) {
                    Some(price) => {
                        ltp = *price as f64;
                    }
                    None => {
                        tracing::error!("Error fetching LTP data from hashmap");
                    }
                }

                // let ltp = client_arc_clone.ltp_data(&ltp_data_req).await.unwrap();
                let ltp_price = ltp as f32;

                let quantity_result = usize::from_str(&order_clone.inner.quantity);

                let position = SmartConnect::new_margin_calculator_position(
                    order_clone.inner.exchange,
                    ProductType::IntraDay,
                    order_clone.transaction_type,
                    &order_clone.inner.symbol_token,
                    ltp_price,
                    quantity_result.unwrap(),
                );

                let margin = match client_arc_clone.calculate_margin(&[position]).await {
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
                        price: ltp_price as f32,
                        margin_required: margin_required as f32,
                    })
                    .await
                    .unwrap();
                return;
            }

            // tracing::info!("\nOrder : {:?} client id : {:?}", &order_clone, &client_id);

            match client_arc_clone.place_order(&order_clone).await {
                Ok(response) => {
                    println!("\n");
                    tracing::info!("Order res => {:?}", response);
                    println!("\n\n");
                    if let Some(is) = response.unique_order_id {
                        tx_main_clone
                            .send(Signal::SetUniqueOrderId(trade_engine_id, is.clone()))
                            .await
                            .unwrap();
                    }
                }
                Err(e) => {
                    tx_main_clone
                        .send(Signal::OrderError(TradeRes {
                            trade_engine_id,
                            trade_id: 0,
                            price: 0.0,
                            strategy: strategy.clone(),
                            order_id: 0,
                        }))
                        .await
                        .unwrap();
                    tracing::error!("Error placing order: {:?}", e);
                }
            };
        }

        /// Associated function to place order to square off open position

        // pub async fn handle_squareoff_placement(
        //     client_id: u32,
        //     client_arc_clone: Arc<SmartConnect>,
        //     order_req: PlaceOrderReq,
        //     tx_main_clone: Arc<Sender<Signal>>,
        //     trade_engine_id: u32,
        //     trade_id: u32,
        //     strategy: Strategy,
        //     remove_trade_engine: bool,
        //     tx_redis: Sender<Signal>,
        //     price_hasmap: HashMap<String, f32>,
        // ) {
        //     // Commented for production
        //     //let order_clone = order_req.clone();
        //     // ----------------------------
        //     let order_clone = order_req.clone();
        //     // info!("Order == {:?}", order_clone);
        //     if client_id == 0 {
        //         let mut ltp: f64 = 0.0;

        //         // let ltp_data_req = LtpDataReq {
        //         //     exchange: order_clone.inner.exchange,
        //         //     trading_symbol: order_clone.clone().inner.trading_symbol,
        //         //     symbol_token: order_clone.inner.symbol_token.clone(),
        //         // };
        //         // let ltp = client_arc_clone.ltp_data(&ltp_data_req).await.unwrap();

        //         // match client_arc_clone.ltp_data(&ltp_data_req).await {
        //         //     Ok(ltp_data) => {
        //         //         ltp = ltp_data.ltp;
        //         //     }
        //         //     Err(e) => {
        //         //         tracing::error!("Error fetching LTP data: {:?}", e);
        //         //         // tx_redis.send(Signal::Error(e)).await.unwrap();
        //         //         // return;
        //         //     }
        //         // }

        //         match price_hasmap.get(&order_clone.inner.symbol_token) {
        //             Some(price) => {
        //                 ltp = *price as f64;
        //             }
        //             None => {
        //                 tracing::error!("Error fetching LTP data from hashmap");
        //             }
        //         }

        //         tx_redis
        //             .send(Signal::UpdateSquareOffDemo {
        //                 client_id,
        //                 trade_id,
        //                 remove_trade_engine,
        //                 strategy: strategy.clone(),
        //                 order: order_clone,
        //                 price: ltp as f32,
        //             })
        //             .await
        //             .unwrap();
        //         return;
        //     }

        //     // let res = client_arc_clone.place_order(&order_req).await.unwrap();

        //     match client_arc_clone.place_order(&order_clone).await {
        //         Ok(response) => {
        //             tracing::info!("Order res => {:?}", response);

        //             if let Some(order_id) = response.unique_order_id {
        //                 let mut ind_status: Option<IndividualOrderStatus> = None;
        //                 let mut check_count = 0;
        //                 loop {
        //                     let is_clone = order_id.clone();
        //                     println!("\n");
        //                     tracing::info!("\nWaiting for 2 seconds.\n");
        //                     sleep(Duration::from_millis(2000));
        //                     ind_status = match client_arc_clone.order_status(is_clone).await {
        //                         Ok(status) => Some(status),
        //                         Err(e) => {
        //                             tracing::error!("Error fetching order status: {:?}", e);
        //                             None
        //                         }
        //                     };
        //                     if let Some(ref status) = ind_status {
        //                         if status.order.status != "open"
        //                             && status.order.order_status != "open"
        //                         {
        //                             break;
        //                         }
        //                     }
        //                     if check_count == 3 {
        //                         tracing::error!("Order status check timed out after 6 seconds.");
        //                         break;
        //                     } else {
        //                         check_count += 1;
        //                     }
        //                 }

        //                 // let ind_status = client_arc_clone.order_status(order_id).await.unwrap();
        //                 // let order = ind_status.clone();
        //                 let order = ind_status.clone().unwrap();
        //                 let order_clone = ind_status.clone().unwrap();
        //                 println!("\n\n");
        //                 tracing::info!("Individual status: {:?}", order);
        //                 println!("\n\n");
        //                 let price = order.order.average_price;

        //                 tx_redis
        //                     .send(Signal::UpdateSquareOff {
        //                         client_id,
        //                         trade_id,
        //                         remove_trade_engine,
        //                         trade_engine_id,
        //                         strategy: strategy.clone(),
        //                         price: price as f32,
        //                         order,
        //                     })
        //                     .await
        //                     .unwrap();
        //                 println!("\n");
        //                 info!("\nIndividual status: {:?}", ind_status);
        //                 // let status = order_clone.order.status.clone();
        //                 // let order_status = order_clone.order.order_status.clone();

        //                 if order_clone.order.status == "rejected"
        //                     || order_clone.order.order_status == "rejected"
        //                 {
        //                     let _ = tx_main_clone
        //                         .send(Signal::OrderRejected(TradeRes {
        //                             trade_engine_id,
        //                             trade_id: 0,
        //                             price: 0.0,
        //                             strategy: strategy.clone(),
        //                         }))
        //                         .await;
        //                 } else if order_clone.order.status == "error"
        //                     || order_clone.order.order_status == "error"
        //                 {
        //                     let _ = tx_main_clone
        //                         .send(Signal::OrderError(TradeRes {
        //                             trade_engine_id,
        //                             trade_id: 0,
        //                             price: 0.0,
        //                             strategy: strategy.clone(),
        //                         }))
        //                         .await;
        //                 }
        //                 // Sleep for 3 seconds before removing trade engine to ensure all processes are complete
        //                 sleep(Duration::from_millis(3000));
        //                 if remove_trade_engine {
        //                     let _ = tx_main_clone
        //                         .send(Signal::RemoveTradeEngine(trade_engine_id))
        //                         .await;
        //                 }
        //             }
        //         }
        //         Err(e) => {
        //             tracing::error!("Error placing square off order {:?}", e);
        //         }
        //     }

        //     // -------------------------------------------------
        // }

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
            price_hasmap: HashMap<String, f32>,
        ) {
            if remove_trade_engine {
                tx_main_clone
                    .send(Signal::SetRemoveTradeEngine(trade_engine_id))
                    .await
                    .unwrap();
            }
            let order_clone = order_req.clone();
            // info!("Order == {:?}", order_clone);
            if client_id == 0 {
                let mut ltp: f64 = 0.0;
                match price_hasmap.get(&order_clone.inner.symbol_token) {
                    Some(price) => {
                        ltp = *price as f64;
                    }
                    None => {
                        tracing::error!("Error fetching LTP data from hashmap");
                    }
                }

                tx_redis
                    .send(Signal::UpdateSquareOffDemo {
                        client_id,
                        trade_id,
                        remove_trade_engine,
                        strategy: strategy.clone(),
                        order: order_clone,
                        price: ltp as f32,
                    })
                    .await
                    .unwrap();
                return;
            }

            // let res = client_arc_clone.place_order(&order_req).await.unwrap();

            match client_arc_clone.place_order(&order_clone).await {
                Ok(response) => {
                    tracing::info!("Order res => {:?}", response);

                    if let Some(order_id) = response.unique_order_id {
                        tx_main_clone
                            .send(Signal::SetUniqueOrderId(trade_engine_id, order_id.clone()))
                            .await
                            .unwrap();

                        // if remove_trade_engine {
                        //     let _ = tx_main_clone
                        //         .send(Signal::RemoveTradeEngine(trade_engine_id))
                        //         .await;
                        // }
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
