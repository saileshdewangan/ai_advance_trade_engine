use std::{
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};

use futures_util::{SinkExt, Stream};
use serde::{de::DeserializeOwned, Serialize};
use tokio::net::TcpStream;
use tokio_tungstenite::{
    connect_async,
    tungstenite::{
        handshake::client::Request,
        protocol::{frame::Frame, CloseFrame},
    },
    MaybeTlsStream,
};

use crate::Error;

/// Type alias for result
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Type alias for WebSocketStream
type WebSocket = tokio_tungstenite::WebSocketStream<MaybeTlsStream<TcpStream>>;

/// Type alias for [`WebSocket`] message
type WsMessage = tokio_tungstenite::tungstenite::Message;

/// Web socket stream to stream the messages
#[pin_project]
pub struct WsStream<M> {
    inner: WebSocket,
    p: PhantomData<M>,
}

impl<M> WsStream<M>
where
    M: TryFrom<Vec<u8>, Error = Error> + DeserializeOwned,
{
    /// Connects to a [`WebSocket`] server.
    pub async fn connect(request: Request) -> Result<Self> {
        debug!("Connecting to webSocket at {}", request.uri());

        let inner = connect_async(request)
            .await
            .map(|(websocket, _)| websocket)?;

        Ok(Self {
            inner,
            p: PhantomData,
        })
    }

    /// Sends the subscription request to the [`WebSocket`]
    pub async fn subscribe<S>(&mut self, message: S) -> Result<()>
    where
        S: Serialize + Send + Sync,
    {
        let msg_str = serde_json::to_string(&message)?;
        trace!("Sending subscribe request {}", msg_str);
        self.inner.send(WsMessage::Text(msg_str)).await?;

        Ok(())
    }

    /// Parses the [`WsMessage`] received from the [`WebSocket`]
    fn parse(msg: Result<WsMessage>) -> Option<Result<M>> {
        match msg {
            Ok(m) => match m {
                WsMessage::Text(txt) => Self::process_text(txt),
                WsMessage::Binary(bin) => Self::process_binary(bin),
                WsMessage::Ping(ping) => Self::process_ping(ping),
                WsMessage::Pong(pong) => Self::process_pong(pong),
                WsMessage::Close(close_frame) => Self::process_close_frame(close_frame),
                WsMessage::Frame(frame) => Self::process_frame(frame),
            },
            Err(e) => Some(Err(e)),
        }
    }

    /// Text message from [`WebSocket`]. Event logged at `trace` level.
    fn process_text(payload: String) -> Option<Result<M>> {
        trace!("Received text payload {payload}");
        Some(serde_json::from_str(&payload).map_err(|e| {
            let msg = format!("Failed to decode websocket text message with error {e}");
            error!("{msg}");
            msg.into()
        }))
    }

    /// Binary message from [`WebSocket`]. Event logged at `trace` level.
    fn process_binary(payload: Vec<u8>) -> Option<Result<M>> {
        trace!("Received binary payload {:?}", payload);
        Some(M::try_from(payload).map_err(|e| {
            let msg = format!("Failed to decode websocket binary message  with error {e}",);
            error!("{msg}");
            msg.into()
        }))
    }

    /// Ping message from [`WebSocket`]. Event logged at `trace` level.
    fn process_ping(payload: Vec<u8>) -> Option<Result<M>> {
        trace!("Ping received at websocket {:?}", payload);
        None
    }

    /// Pong message from [`WebSocket`]. Event logged at `trace` level.
    fn process_pong(payload: Vec<u8>) -> Option<Result<M>> {
        trace!("Pong received at websocket {:?}", payload);
        None
    }

    /// CloseFrame message from [`WebSocket`]. Event logged at `trace` level.
    fn process_close_frame(close_frame: Option<CloseFrame>) -> Option<Result<M>> {
        let msg = format!("CloseFrame request from websocket {:?}", close_frame);
        trace!("{msg}");
        Some(Err(msg.into()))
    }

    /// Frame message from [`WebSocket`]. Event logged at `trace` level.
    fn process_frame(frame: Frame) -> Option<Result<M>> {
        trace!("Frame message at websocket {:?}", frame);
        None
    }
}

impl<M> From<WebSocket> for WsStream<M> {
    fn from(inner: WebSocket) -> Self {
        Self {
            inner,
            p: PhantomData,
        }
    }
}

impl<M> Into<WebSocket> for WsStream<M> {
    fn into(self) -> WebSocket {
        self.inner
    }
}

impl<M> Stream for WsStream<M>
where
    M: TryFrom<Vec<u8>, Error = Error> + DeserializeOwned,
{
    type Item = Result<M>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            let input = match Pin::new(self.as_mut().project().inner).poll_next(cx) {
                Poll::Ready(Some(input)) => input.map_err(|e| e.into()),
                Poll::Ready(None) => return Poll::Ready(None),
                Poll::Pending => return Poll::Pending,
            };

            match Self::parse(input) {
                Some(m) => return Poll::Ready(Some(m)),
                None => continue,
            }
        }
    }
}










// #![allow(unused)]

// use std::{
//     marker::PhantomData,
//     pin::Pin,
//     sync::Arc,
//     task::{Context, Poll},
// };

// use futures_util::{SinkExt, Stream};
// use serde::{de::DeserializeOwned, Serialize};
// use tokio::{
//     net::TcpStream,
//     sync::Mutex,
//     time::{interval, Duration},
// };
// use tokio_tungstenite::{
//     connect_async,
//     tungstenite::{
//         handshake::client::Request,
//         protocol::{frame::Frame, CloseFrame},
//     },
//     MaybeTlsStream,
// };

// use crate::Error;

// /// Type alias for result
// type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

// /// Type alias for WebSocketStream
// type WebSocket = tokio_tungstenite::WebSocketStream<MaybeTlsStream<TcpStream>>;

// /// Type alias for [`WebSocket`] message
// type WsMessage = tokio_tungstenite::tungstenite::Message;

// /// Web socket stream to stream the messages
// #[pin_project::pin_project]
// pub struct WsStream<M> {
//     inner: Arc<Mutex<WebSocket>>,
//     p: PhantomData<M>,
// }

// impl<M> WsStream<M>
// where
//     M: TryFrom<Vec<u8>, Error = Error> + DeserializeOwned,
// {
//     /// Connects to a [`WebSocket`] server and starts a ping task to keep the connection alive.
//         /// Connects to a [`WebSocket`] server.
//         pub async fn connect(request: Request) -> Result<Self> {
//             debug!("Connecting to webSocket at {}", request.uri());
    
//             let inner = connect_async(request)
//                 .await
//                 .map(|(websocket, _)| websocket)?;
    
//             Ok(Self {
//                 inner: Arc::new(Mutex::new(inner)), // Initialize Arc<Mutex>
//                 p: PhantomData,
//             })
//         }

//   /// Starts a task that sends ping messages at regular intervals.
//   pub fn start_pinging(&self, ping_interval_duration: Duration) {
//     let inner_clone = Arc::clone(&self.inner); // Clone the Arc

//     tokio::spawn(async move {
//         let mut ping_interval = interval(ping_interval_duration);
//         loop {
//             ping_interval.tick().await; // Wait for the next ping interval

//             let mut ws = inner_clone.lock().await; // Acquire the lock
//             if let Err(e) = ws.send(WsMessage::Ping(vec![])).await {
//                 eprintln!("Failed to send ping: {:?}", e);
//                 break; // Exit the loop if ping fails
//             }
//             println!("Ping sent");
//         }
//     });
// }
//     /// Sends the subscription request to the [`WebSocket`]
//     pub async fn subscribe<S>(&mut self, message: S) -> Result<()>
//     where
//         S: Serialize + Send + Sync,
//     {
//         let msg_str = serde_json::to_string(&message)?;
//         trace!("Sending subscribe request {}", msg_str);
//         let mut ws = self.inner.lock().await;
//         ws.send(WsMessage::Text(msg_str)).await?;

//         Ok(())
//     }

//     /// Parses the [`WsMessage`] received from the [`WebSocket`]
//     fn parse(msg: Result<WsMessage>) -> Option<Result<M>> {
//         match msg {
//             Ok(m) => match m {
//                 WsMessage::Text(txt) => Self::process_text(txt),
//                 WsMessage::Binary(bin) => Self::process_binary(bin),
//                 WsMessage::Ping(ping) => Self::process_ping(ping),
//                 WsMessage::Pong(pong) => Self::process_pong(pong),
//                 WsMessage::Close(close_frame) => Self::process_close_frame(close_frame),
//                 WsMessage::Frame(frame) => Self::process_frame(frame),
//             },
//             Err(e) => Some(Err(e)),
//         }
//     }

//     /// Text message from [`WebSocket`]. Event logged at `trace` level.
//     fn process_text(payload: String) -> Option<Result<M>> {
//         trace!("Received text payload {payload}");
//         Some(serde_json::from_str(&payload).map_err(|e| {
//             let msg = format!("Failed to decode websocket text message with error {e}");
//             error!("{msg}");
//             msg.into()
//         }))
//     }

//     /// Binary message from [`WebSocket`]. Event logged at `trace` level.
//     fn process_binary(payload: Vec<u8>) -> Option<Result<M>> {
//         trace!("Received binary payload {:?}", payload);
//         Some(M::try_from(payload).map_err(|e| {
//             let msg = format!("Failed to decode websocket binary message  with error {e}",);
//             error!("{msg}");
//             msg.into()
//         }))
//     }

//     /// Ping message from [`WebSocket`]. Event logged at `trace` level.
//     fn process_ping(payload: Vec<u8>) -> Option<Result<M>> {
//         trace!("Ping received at websocket {:?}", payload);
//         None
//     }

//     /// Pong message from [`WebSocket`]. Event logged at `trace` level.
//     fn process_pong(payload: Vec<u8>) -> Option<Result<M>> {
//         trace!("Pong received at websocket {:?}", payload);
//         None
//     }

//     /// CloseFrame message from [`WebSocket`]. Event logged at `trace` level.
//     fn process_close_frame(close_frame: Option<CloseFrame>) -> Option<Result<M>> {
//         let msg = format!("CloseFrame request from websocket {:?}", close_frame);
//         trace!("{msg}");
//         Some(Err(msg.into()))
//     }

//     /// Frame message from [`WebSocket`]. Event logged at `trace` level.
//     fn process_frame(frame: Frame) -> Option<Result<M>> {
//         trace!("Frame message at websocket {:?}", frame);
//         None
//     }
// }

// impl<M> From<WebSocket> for WsStream<M> {
//     fn from(inner: WebSocket) -> Self {
//         Self {
//             inner: Arc::new(Mutex::new(inner)),
//             p: PhantomData,
//         }
//     }
// }

// impl<M> Stream for WsStream<M>
// where
//     M: TryFrom<Vec<u8>, Error = Error> + DeserializeOwned,
// {
//     type Item = Result<M>;

//     fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
//         // First, we need to lock the inner WebSocket mutex.
//         let mut ws_lock = match self.inner.try_lock() {
//             Ok(guard) => guard,             // Successfully acquired the lock
//             Err(_) => return Poll::Pending, // Could not acquire lock, so return Pending
//         };

//         // Pin the WebSocketStream for safe polling
//         let mut ws = Pin::new(&mut *ws_lock);

//         match ws.poll_next(cx) {
//             Poll::Ready(Some(input)) => {
//                 let input = input.map_err(|e| e.into());
//                 match Self::parse(input) {
//                     Some(m) => Poll::Ready(Some(m)),
//                     None => Poll::Pending,
//                 }
//             }
//             Poll::Ready(None) => Poll::Ready(None),
//             Poll::Pending => Poll::Pending,
//         }
//     }
// }
