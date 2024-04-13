use thiserror::Error;
use tower_service::Service;

mod messages;
mod request;
mod response;

pub use messages::AdvanceStateMetadata;
pub use request::Request;
pub use response::Response;

#[derive(Error, Debug)]
pub enum Error<E> {
    #[error("Hyper error: {0}")]
    Hyper(#[from] hyper::Error),
    #[error("Deserialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("Deserialization hex parsing error: {0}")]
    HexParseError(#[from] hex::FromHexError),
    #[error("Invalid URI: {0}")]
    InvalidUri(#[from] hyper::http::uri::InvalidUri),
    #[error("Cartesi Service Error: {0}")]
    ServiceError(E),
}

pub async fn listen_http<S>(service: &mut S, host_uri: &str) -> Result<(), Error<S::Error>>
where
    S: Service<Request, Response = Response>,
{
    let client = hyper::Client::new();

    let mut response = Response::empty_accept();
    loop {
        // set the finish message and get the new request
        let finish_http_request = response.finish_message().build_http_request(host_uri);
        let resp = client.request(finish_http_request).await?;
        if resp.status() == hyper::StatusCode::ACCEPTED {
            tracing::info!("No pending rollup request, trying again");
            continue; // no pending rollup request so run the loop again
        }
        let body_bytes = hyper::body::to_bytes(resp.into_body()).await?;
        let rollup_request: messages::RollupRequest = serde_json::from_slice(&body_bytes)?;
        let request = Request::try_from(rollup_request)?;

        // let the dapp process the request
        response = service.call(request).await.map_err(Error::ServiceError)?;

        // handle the additional calls as required by the dApp outputs
        for output in response.outputs.iter() {
            tracing::info!("Sending output {:?}", output);
            let resp = client.request(output.build_http_request(host_uri)).await?;
            tracing::info!("Output response: {:?}", resp.status());
        }
    }
}
