// Service that wraps StateRead queries and routes them to the Cartesi machine via the /inspect HTTP endpoint

use std::{net::SocketAddr, pin::Pin};

use futures_util::{Future, FutureExt};
use hyper::{body::Buf, Uri};

pub struct InspectStateReader(SocketAddr);

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

    fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        Ok(()).into()
    }

    fn call(&mut self, req: zebra_state::ReadRequest) -> Self::Future {
        let client = hyper::Client::new();
        let mut state_query_bytes = Vec::new();
        ciborium::into_writer(&req, &mut state_query_bytes).unwrap();
        let uri = Uri::builder()
            .scheme("http")
            .authority(self.0.to_string())
            .path_and_query(format!(
                "/inspect/{}",
                base64_url::encode(&state_query_bytes)
            ))
            .build()
            .unwrap();

        tracing::info!("Sending inspect request: {}", uri);

        async move {
            let request = hyper::Request::builder()
                .method(hyper::Method::GET)
                .uri(uri)
                .body(hyper::Body::empty())
                .unwrap();
            let response = client.request(request).await?;
            let body = hyper::body::to_bytes(response).await?;

            tracing::info!("Inspect response: {:?}", body);

            // hacky - no error handling or anything here
            let utf = std::str::from_utf8(&body)?;
            let resp_json = json::parse(utf)?;
            if let Some(hex) = resp_json["reports"][0]["payload"].as_str() {
                let bytes = hex::decode(hex.trim_start_matches("0x"))?;

                let state_query_response = ciborium::from_reader(bytes.as_slice()).unwrap();
                Ok(state_query_response)
            } else {
                Err(zebra_state::BoxError::from("No payload in response"))
            }
        }
        .boxed()
    }
}
