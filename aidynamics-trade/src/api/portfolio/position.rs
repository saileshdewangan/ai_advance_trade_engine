use crate::types::{ExchangeType, ProductType};

/// Placeholder containing Position information
#[allow(missing_docs)]
#[derive(Debug, Deserialize,Serialize)]
#[api(GET, Position)]
pub struct Position {
    pub exchange: ExchangeType,
    #[serde(rename = "symboltoken")]
    pub symbol_token: String,
    #[serde(rename = "producttype")]
    pub product_type: ProductType,
    #[serde(rename = "tradingsymbol")]
    pub trading_symbol: String,
    #[serde(rename = "symbolname")]
    pub symbol_name: String,
    #[serde(rename = "instrumenttype")]
    pub instrument_type: String,
    #[serde(rename = "priceden")]
    pub price_den: String,
    #[serde(rename = "pricenum")]
    pub price_num: String,
    #[serde(rename = "genden")]
    pub gen_den: String,
    #[serde(rename = "gennum")]
    pub gen_num: String,
    #[serde(rename = "precision")]
    pub precision: String,
    #[serde(rename = "multiplier")]
    pub multiplier: String,
    #[serde(rename = "boardlotsize")]
    pub board_lot_size: String,
    #[serde(rename = "buyquantity")]
    pub buy_quantity: String,
    #[serde(rename = "sellquantity")]
    pub sell_quantity: String,
    #[serde(rename = "buyamount")]
    pub buy_amount: String,
    #[serde(rename = "sellamount")]
    pub sell_amount: String,
    #[serde(rename = "symbolgroup")]
    pub symbol_group: String,
    #[serde(rename = "strikeprice")]
    pub strike_price: String,
    #[serde(rename = "optiontype")]
    pub option_type: String,
    #[serde(rename = "expirydate")]
    pub expiry_date: String,
    #[serde(rename = "lotsize")]
    pub lot_size: String,
    #[serde(rename = "cfbuyqty")]
    pub cf_buy_qty: String,
    #[serde(rename = "cfsellqty")]
    pub cf_sell_qty: String,
    #[serde(rename = "cfbuyamount")]
    pub cf_buy_amount: String,
    #[serde(rename = "cfsellamount")]
    pub cf_sell_amount: String,
    #[serde(rename = "buyavgprice")]
    pub buy_average_price: String,
    #[serde(rename = "sellavgprice")]
    pub sell_average_price: String,
    #[serde(rename = "avgnetprice")]
    pub average_net_price: String,
    #[serde(rename = "netvalue")]
    pub net_value: String,
    #[serde(rename = "netqty")]
    pub net_qty: String,
    #[serde(rename = "totalbuyvalue")]
    pub total_buy_value: String,
    #[serde(rename = "totalsellvalue")]
    pub total_sell_value: String,
    #[serde(rename = "cfbuyavgprice")]
    pub cf_buy_average_price: String,
    #[serde(rename = "cfsellavgprice")]
    pub cf_sell_average_price: String,
    #[serde(rename = "totalbuyavgprice")]
    pub total_buy_average_price: String,
    #[serde(rename = "totalsellavgprice")]
    pub total_sell_average_price: String,
    #[serde(rename = "netprice")]
    pub net_price: String,
}
