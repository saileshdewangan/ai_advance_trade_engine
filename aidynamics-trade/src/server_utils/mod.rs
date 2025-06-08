mod api;
pub use api::OrderPlacedType;

/// End points defined of web server
pub mod end_points;
// pub use end_points::EndPoint;

mod server_http;
pub use server_http::ServerHttp;

// mod response;
// pub use response::Response;

// mod header;
// pub use header::HttpHeader;