// Service that wraps StateRead queries and routes them to the Cartesi machine via the /inspect HTTP endpoint

use std::{net::SocketAddr, pin::Pin};

use futures_util::{Future, FutureExt};
use hyper::body::Buf;

struct InspectStateReader(SocketAddr);

impl InspectStateReader {
    pub fn new(addr: SocketAddr) -> Self {
        Self(addr)
    }
}

impl tower::Service<zebra_state::ReadRequest> for InspectStateReader {
    type Response = zebra_state::ReadResponse;
    type Error = zebra_state::BoxError;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        Ok(()).into()
    }

    fn call(&mut self, req: zebra_state::ReadRequest) -> Self::Future {
        let client = hyper::Client::new();
        let mut state_query_bytes = Vec::new();
        ciborium::into_writer(&req, &mut state_query_bytes).unwrap();
        let url = self.0.clone();

        async move {
            let request = hyper::Request::builder()
                .method(hyper::Method::GET)
                .uri(format!("{}/inspect/{}", url, base64_url::encode(&state_query_bytes)))
                .body(hyper::Body::empty())
                .unwrap();
            let response = client.request(request).await?;
            let body = hyper::body::to_bytes(response).await?;

            let state_query_response = ciborium::from_reader(&mut body.reader()).unwrap();
            Ok(state_query_response)

        }.boxed()
    }
}