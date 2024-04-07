use std::{error::Error, future::Future, pin::Pin, task::Poll};
use tower_service::Service;

mod messages;
mod request;
mod response;

pub use messages::AdvanceStateMetadata;
pub use request::Request;
pub use response::Response;

pub type BoxError = Box<dyn Error + Send + Sync + 'static>;
pub trait CartesiRollApp {
    fn handle_advance_state(
        &mut self,
        metadata: messages::AdvanceStateMetadata,
        payload: Vec<u8>,
    ) -> Pin<Box<dyn Future<Output = Result<Response, Box<dyn Error + Send + Sync + 'static>>> + Send>>;
    fn handle_inspect_state(
        &mut self,
        payload: Vec<u8>,
    ) -> Pin<Box<dyn Future<Output = Result<Response, Box<dyn Error + Send + Sync + 'static>>> + Send>>;
}

pub struct CartesiService<S> {
    inner: S,
}

impl<S: CartesiRollApp> CartesiService<S> {
    pub fn new(inner: S) -> Self {
        Self { inner }
    }

    pub async fn listen_http(&mut self, host_uri: &str) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
        let client = hyper::Client::new();

        let mut response = Response::empty_accept();
        loop {
            // set the finish message and get the new request
            let finish_http_request = response
                .finish_message()
                .build_http_request(host_uri.try_into()?);
            let resp = client.request(finish_http_request).await?;
            if resp.status() == hyper::StatusCode::ACCEPTED {
                tracing::info!("No pending rollup request, trying again");
                continue; // no pending rollup request so run the loop again
            }
            let body_bytes = hyper::body::to_bytes(resp.into_body()).await?;
            let rollup_request: messages::RollupRequest = serde_json::from_slice(&body_bytes)?;
            let request = Request::try_from(rollup_request)?;

            // let the dapp process the request
            response = self.call(request).await?;

            // handle the additional calls as required by the dApp outputs
            for output in response.outputs.iter() {
                tracing::info!("Sending output {:?}", output);
                let resp = client
                    .request(output.build_http_request(host_uri.try_into()?))
                    .await?;
                tracing::info!("Output response: {:?}", resp.status());
            }
        }
    }
}

impl<S: CartesiRollApp> Service<Request> for CartesiService<S> {
    type Response = Response;
    type Error = Box<dyn Error + Send + Sync + 'static>;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request) -> Self::Future {
        match req {
            Request::AdvanceState { metadata, payload } => {
                self.inner.handle_advance_state(metadata, payload)
            }
            Request::InspectState { payload } => self.inner.handle_inspect_state(payload),
        }
    }
}
