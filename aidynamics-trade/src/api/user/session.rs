
use aidynamics_trade_utils::sys::otp;

use crate::Result;

/// Session request to the API
#[derive(Debug, Serialize)]
#[api(POST, Login)]
pub struct SessionReq {
    /// Client code
    #[serde(rename = "clientcode")]
    pub client_code: String,
    /// Password
    pub password: String,
    /// Current OTP
    pub totp: String,
}

/// Session response received on calling the Login endpoint
#[derive(Debug, Deserialize,Serialize)]
pub struct SessionRes {
    /// JWT token
    #[serde(rename = "jwtToken")]
    pub jwt_token: String,
    /// Refresh token
    #[serde(rename = "refreshToken")]
    pub refresh_token: String,
    /// Feed token
    #[serde(rename = "feedToken")]
    pub feed_token: String,
}

impl SessionReq {
    /// Authenticate to get open APIs trading access using AngelOne Ltd. Account Id
    /// session established via SmartAPI remains active until 5 am on the following day,
    /// unless the user chooses to log out
    pub async fn new<C, P, O>(client_code: C, pin: P, otp_token: O) -> Result<Self>
    where
        C: Into<String>,
        P: Into<String>,
        O: Into<String>,
    {
        let totp = otp(otp_token)?;

        Ok(
            Self {
            client_code: client_code.into(),
            password: pin.into(),
            totp,
        })
    }

    /// Authenticate to get open APIs trading access using AngelOne Ltd. Account Id
    /// session established via SmartAPI remains active until 5 am on the following day,
    /// unless the user chooses to log out
    /// THIS IS CUSTOMIZED FUNCTION TO DIRECTLY PASS : TOTP
    pub async fn new_with_totp<C, P, O>(client_code: C, pin: P, totp: O) -> Result<Self>
    where
        C: Into<String>,
        P: Into<String>,
        O: Into<String>,
    {
        Ok(Self {
            client_code: client_code.into(),
            password: pin.into(),
            totp:totp.into(),
        })
    }
}
