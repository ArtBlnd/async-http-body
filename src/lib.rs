mod adapter;

pub use adapter::AsyncBodyAdapter;
pub use async_trait::async_trait as async_body;

use bytes::Buf;
use http::HeaderMap;

use std::pin::Pin;

#[async_body]
pub trait AsyncBody {
    type Data: Buf;
    type Error;

    async fn data(self: Pin<&mut Self>) -> Option<Result<Self::Data, Self::Error>>;
    async fn trailers(self: Pin<&mut Self>) -> Result<Option<HeaderMap>, Self::Error>;
}
