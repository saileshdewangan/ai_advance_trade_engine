use std::fmt::Display;

use reqwest::{redirect::Policy, Client, ClientBuilder, IntoUrl, Method};
use serde::{de::DeserializeOwned, Serialize};

use crate::Result;

use super::{EndPoint, HttpHeader, Response};

/// Placeholder for the Http client
#[derive(Debug)]
pub struct HttpClient {
    /// Inner client
    client: Client,
    /// JWT token for bearer
    jwt_token: Option<String>,
    /// Header map that can be modified
    http_header: HttpHeader, // Add this field
}

impl HttpClient {
    /// Returns a new instance for the http client
    /// Returns a new instance for the http client
    pub async fn new<A>(api_key: A) -> Result<Self>
    where
        A: AsRef<str>,
    {
        let http_header = HttpHeader::new(api_key.as_ref(), None::<String>).await?;

        let client = ClientBuilder::new()
            .redirect(Policy::custom(|a| a.follow()))
            .default_headers(http_header.clone().into_inner()) // Clone and convert
            .build()
            .map_err(|e| {
                error!("Failed to create http client {e}");
                e
            })?;

        Ok(Self {
            client,
            jwt_token: None,
            http_header, // Store the HttpHeader instance
        })
    }

    /// Returns a new instance for the http client with an initial JWT token.
    pub async fn new_with_jwt<A, J>(api_key: A, jwt_token: Option<&J>) -> Result<Self>
    where
        A: AsRef<str>,
        J: Display,
    {
        let http_header = HttpHeader::new(api_key.as_ref(), jwt_token.as_ref()).await?; // Pass jwt_token

        let client = ClientBuilder::new()
            .redirect(Policy::custom(|a| a.follow()))
            .default_headers(http_header.clone().into_inner())
            .build()
            .map_err(|e| {
                error!("Failed to create http client {e}");
                e
            })?;

        Ok(Self {
            client,
            jwt_token: None, // You might want to store the token here if needed.
            http_header,
        })
    }

    /// Sets the jwt token for authorization header
    pub fn jwt_token<J>(&mut self, jwt_token: J)
    where
        J: Into<String>,
    {
        self.jwt_token = Some(jwt_token.into());
    }

    /// Sets the Authorization and X-PrivateKey headers using HttpHeader methods.
    pub fn set_auth_and_key(&mut self, token: &str, key: &str) -> Result<()> {
        let mut inner_headers = self.http_header.clone().into_inner(); // Get inner HeaderMap

        // inner_headers.insert(
        //     header::AUTHORIZATION,
        //     format!("Bearer {}", token).parse().unwrap(),
        // );

        self.jwt_token = Some(token.into());

        inner_headers.insert("X-PrivateKey", key.parse().unwrap());

        // Recreate HttpHeader, because it owns the HeaderMap
        self.http_header = HttpHeader(inner_headers);

        Ok(())
    }

    /// Makes the http request
    pub async fn request<B, R>(&self, method: Method, ep: EndPoint, body: &B) -> Result<Response<R>>
    where
        B: Serialize + ?Sized,
        R: DeserializeOwned + std::fmt::Debug,
    {
        debug!("New {method} request for {ep}");

        let request = match method {
            Method::GET => self.client.get(ep.url()).query(body),
            Method::POST => self.client.post(ep.url()).json(body),
            _ => unimplemented!(),
        };

        let request = match self.jwt_token.as_ref() {
            Some(token) => request.bearer_auth(token),
            None => request,
        };

        let req_res = request.send().await.map_err(|e| {
            error!("{method} request to {ep} failed: {e}");
            e
        })?;

        let res: Response<R> = req_res.json().await.map_err(|e| {
            error!("{ep} {e}");
            e
        })?;

        if res.status == false {
            error!("{method} request to {ep} failed: {}", res.message);
            return Err(res.message.into());
        }

        debug!("{method} request to {ep} open");

        Ok(res)
    }

    /// Makes the get request
    pub async fn get_json_url<U, R>(url: U) -> Result<R>
    where
        U: IntoUrl,
        R: DeserializeOwned + std::fmt::Debug,
    {
        let res = reqwest::get(url).await?.json().await?;
        Ok(res)
    }

    /// Makes the get request
    pub async fn get<B, R>(&self, ep: EndPoint, body: &B) -> Result<Response<R>>
    where
        B: Serialize + ?Sized,
        R: DeserializeOwned + std::fmt::Debug,
    {
        self.request(Method::GET, ep, body).await
    }

    /// Makes the post request
    pub async fn post<B, R>(&self, ep: EndPoint, body: &B) -> Result<Response<R>>
    where
        B: Serialize + ?Sized,
        R: DeserializeOwned + std::fmt::Debug,
    {
        self.request(Method::POST, ep, body).await
    }
}
