use aidynamics_trade_utils::Error;
use serde_repr::{Deserialize_repr, Serialize_repr};

/// Exchange type for subscription
#[derive(Debug, Serialize_repr, Deserialize_repr, PartialEq, Eq, Clone, Copy)]
#[repr(u8)]
pub enum SubscriptionMode {
    /// Last traded price
    Ltp = 1,
    /// Quote data
    Quote = 2,
    /// Snap quote data
    SnapQuote = 3,
    /// Depth
    Depth = 4,
}

impl Default for SubscriptionMode {
    fn default() -> Self {
        Self::Ltp
    }
}

impl TryFrom<u8> for SubscriptionMode {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        let _self = match value {
            1 => Self::Ltp,
            2 => Self::Quote,
            3 => Self::SnapQuote,
            4 => Self::Depth,
            _ => return Err("Invalid Subscription Mode".into()),
        };
        Ok(_self)
    }
}
