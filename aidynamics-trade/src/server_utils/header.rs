use std::fmt::Display;

use reqwest::header::{self, HeaderMap};

use crate::{
    // sys::{local_ip, mac_addr, public_ip},
    Result,
};

/// Http headers to be sent to the API
pub struct HttpHeader(HeaderMap);

impl HttpHeader {
    /// Returns a new instance for the http headers
    pub async fn new<A, J>(jwt_token: Option<J>) -> Result<Self>
    where
        A: AsRef<str>,
        J: Display,
    {
        let mut headers = HeaderMap::new();

        headers.insert(header::CONTENT_TYPE, "application/json".parse().unwrap());
        headers.insert(header::ACCEPT, "application/json".parse().unwrap());
        headers.insert("X-UserType", "USER".parse().unwrap());
        headers.insert("X-SourceID", "WEB".parse().unwrap());

        if let Some(token) = jwt_token {
            headers.insert(
                header::AUTHORIZATION,
                format!("Bearer {token}").parse().unwrap(),
            );
        }

        Ok(Self(headers))
    }

    /// Returns the inner HeaderMap
    pub fn into_inner(self) -> HeaderMap {
        self.0
    }
}
