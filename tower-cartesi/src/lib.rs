use http::Uri;
use std::{error::Error, future::Future, pin::Pin, task::Poll};
use tower_service::Service;

mod messages;
mod request;
mod response;

pub use request::Request;
pub use response::Response;

pub trait CartesiRollApp {
    fn handle_advance_state(
        &mut self,
        metadata: messages::AdvanceStateMetadata,
        payload: Vec<u8>,
    ) -> impl Future<Output = Result<Response, Box<dyn Error>>> + Send + 'static;
    fn handle_inspect_state(&mut self, payload: Vec<u8>) -> impl Future<Output = Result<Response, Box<dyn Error>>> + Send + 'static;
}

pub struct CartesiService<S> {
    inner: S,
}

impl<S: CartesiRollApp> CartesiService<S> {
    pub fn new(inner: S) -> Self {
        Self { inner }
    }

    pub async fn listen_http<T>(uri: T) -> Result<(), Box<dyn Error>>
    where
        Uri: TryFrom<T>,
        <Uri as TryFrom<T>>::Error: Into<Box<dyn Error>>,
    {
        let client = hyper::Client::new();
        let mut response = Response::empty_accept();
        loop {
            // send the requests for the notices, reports and vouchers
            
        }
    }
}

impl<S: CartesiRollApp> Service<Request> for CartesiService<S> {
    type Response = Response;
    type Error = Box<dyn Error>;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request) -> Self::Future {
        match req {
            Request::AdvanceState { metadata, payload } => {
                Box::pin(self.inner.handle_advance_state(metadata, payload))
            }
            Request::InspectState { payload } => {
                Box::pin(self.inner.handle_inspect_state(payload))
            }
        }
    }
}
