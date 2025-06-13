/// Module to implement full client structs
/// Module to create a new websocket connection
// pub mod trade_handler;
/// Module to create a trade engine
pub mod trade_engine;
/// Represents a single client's container that manages multiple TradeEngines.
///
/// A `ClientNode` encapsulates all trade-related engines specific to a single client.
/// Each `TradeEngine` inside is identified by a unique `ClientId` (which could refer to strategy ID, engine type, or symbol-specific logic).
///
/// ## Fields
/// - `trade_engines`: A `HashMap` mapping `ClientId` to corresponding `TradeEngine` instances.
///
/// ## Example Usage:
/// ```rust
/// let mut client_node = ClientNode {
///     trade_engines: HashMap::new(),
/// };
///
/// client_node.trade_engines.insert(client_id, TradeEngine::new());
/// ```
pub mod client_node;
