/// Risk Management System returns fund, cash and margin information of the user for equity and commodity segments
#[allow(missing_docs)]
#[derive(Debug, Deserialize,Serialize)]
#[api(GET, RmsLimit)]
pub struct Rms {
    pub net: String,
    #[serde(rename = "availablecash")]
    pub available_cash: String,
    #[serde(rename = "availableintradaypayin")]
    pub available_intra_day_pay_in: String,
    #[serde(rename = "availablelimitmargin")]
    pub available_limit_margin: String,
    pub collateral: String,
    #[serde(rename = "m2munrealized")]
    pub m2m_unrealized: String,
    #[serde(rename = "m2mrealized")]
    pub m2m_realized: String,
    #[serde(rename = "utiliseddebits")]
    pub utilized_debits: String,
    #[serde(rename = "utilisedspan")]
    pub utilized_span: Option<String>,
    #[serde(rename = "utilisedoptionpremium")]
    pub utilized_option_premium: Option<String>,
    #[serde(rename = "utilisedholdingsales")]
    pub utilized_holding_sales: Option<String>,
    #[serde(rename = "utilisedexposure")]
    pub utilized_exposure: Option<String>,
    #[serde(rename = "utilisedturnover")]
    pub utilized_turnover: Option<String>,
    #[serde(rename = "utilisedpayout")]
    pub utilized_payout: Option<String>,
}
