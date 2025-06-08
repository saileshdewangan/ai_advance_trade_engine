use crate::types::TransactionType;

#[derive(Debug, Clone)]
/// Firebase base data added
pub struct FirebaseTrade {
    /// Current market price
    pub cmp: f64,

    /// Added date time
    pub datetime: String,

    /// Execution time
    pub executiontime: String,

    /// expiry date
    pub expiry: String,

    /// Is hidden
    pub hidden: u32,

    /// status of trade 0 = waiting, 1 = executed , 2 = cancelled
    pub status: u32,

    /// Strategy id
    pub strategyid: u32,

    /// symbol
    pub symbol: String,

    /// transaction type
    pub transactiontype: TransactionType,

    /// trigger price
    pub triggerprice: f64,
}
