use std::fmt;

/// Base url
const BASE_URL: &str = "http://localhost:3000";
/// Trades url
pub const TRADES: &str = "trades";
/// Clients url
pub const CLIENTS: &str = "clients";
/// Updates url
pub const UPDATES: &str = "updates";
/// Orders url
pub const ORDER: &str = "order";

/// All server APIs
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum EndPoint {
    /// Trades api
    Trades,
    /// Clients api
    Clients,
    /// Updates api
    Updates,
    /// Orders api
    Orders,
}

/// Trades URL
pub fn trades_url() -> String {
    format!("{}/{}", BASE_URL, TRADES)
}

/// Clients URL
pub fn clients_url() -> String {
    format!("{}/{}", BASE_URL, CLIENTS)
}

/// Updates URL
pub fn updates_url() -> String {
    format!("{}/{}", BASE_URL, UPDATES)
}

/// Order URL
pub fn order_url() -> String {
    format!("{}/{}", BASE_URL, ORDER)
}

/// Implement Display for EndPoint
impl fmt::Display for EndPoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let url = match self {
            EndPoint::Trades => trades_url(),
            EndPoint::Clients => clients_url(),
            EndPoint::Updates => updates_url(),
            EndPoint::Orders => order_url(),
        };
        write!(f, "{}", url)
    }
}
