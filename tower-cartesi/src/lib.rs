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

/// Repeatedly poll the given host for new requests
/// This works in standalone (no-backend) mode outside the Cartesi machine
/// and also works in the Cartesi machine when the http rollup interface is used
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

use graphql_client::GraphQLQuery;

// The paths are relative to the directory where your `Cargo.toml` is located.
// Both json and the GraphQL schema language are supported as sources for the schema
#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/schema.graphql",
    query_path = "graphql/inputs_query.graphql",
    response_derives = "Debug"
)]
pub struct InputsQuery;

/// Poll a graphql interface for new inputs
/// This will NOT work inside the Cartesi machine
/// It is indended to be used alongside a running machine to receive the same inputs
pub async fn listen_graphql<S>(
    service: &mut S,
    host_uri: &str,
    page_size: usize,
) -> Result<(), Error<S::Error>>
where
    S: Service<Request, Response = Response>,
{
    let client = hyper::Client::new();
    let mut cursor = None;

    loop {
        let request_body = InputsQuery::build_query(inputs_query::Variables {
            first: page_size as i64,
            after: cursor.clone(),
        });
        let request = hyper::Request::builder()
            .uri(host_uri)
            .method("POST")
            .header("Content-Type", "application/json")
            .body(hyper::Body::from(
                serde_json::to_string(&request_body).unwrap(),
            ))
            .unwrap();
        let resp = client.request(request).await?;
        let response_body: graphql_client::Response<inputs_query::ResponseData> =
            serde_json::from_reader(&hyper::body::to_bytes(resp.into_body()).await?[..])?;

        for edge in response_body.data.unwrap().inputs.edges.into_iter() {
            cursor = Some(edge.cursor);
            service
                .call(edge.node.try_into().unwrap())
                .await
                .map_err(Error::ServiceError)?;
        }
    }
}
