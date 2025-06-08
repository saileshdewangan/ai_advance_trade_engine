/// Transaction Types
#[derive(Debug,PartialEq, Serialize, Deserialize, Clone, Copy)]
pub enum TransactionType {

// this is original code Buy and Sell is small case
    // /// Buy
    // Buy,
    // /// Sell
    // Sell,


// Edited code

/// Buy
BUY,
/// Sell
SELL,
}

impl Default for TransactionType {
    fn default() -> Self {
        Self::BUY
    }
}
