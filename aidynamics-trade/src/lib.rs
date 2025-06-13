//! Crate contains Angel One API SDK data structures and calls
#![forbid(unsafe_code)]
#![deny(unused_imports)]
#![deny(unused_variables)]
#![deny(missing_docs)]
#![deny(clippy::all)]

#[macro_use]
extern crate aidynamics_trade_derive;

#[macro_use]
extern crate serde;

pub use aidynamics_trade_utils::Result;

pub use firebase_rs;

mod smart_connect;

pub use smart_connect::SmartConnect;

mod api;
pub use api::{funds, gtt, market, order, portfolio, user, ws};

/// Various types for Angel One API SDK
pub mod types;

mod client_utils;
pub use client_utils::trade_handler;
pub use client_utils::trade_engine;
pub use client_utils::client_node;


mod server_utils;
pub use server_utils::{OrderPlacedType,end_points};

/// Websocket Client library
pub mod websocket;

#[cfg(test)]
pub mod test_util;

/// Redis Utils implementation
pub mod redis_utils;
pub use redis_utils::RedisUtils;

/// Firebase Utils implementation
pub mod firebase_utils;
pub use firebase_utils::FirebaseTrade;

/// Trade Engine Processor
// pub mod trade_engine_processor;


/// Trade Engine Processor
// pub mod advanced_trade_engine_processor;

/// Redis stream producer
// pub mod redis_stream_producer;

/// Order processor
pub mod order_processor;


