use inputs_query::CompletionStatus;
use thiserror::Error;
use tokio::time::interval;
use tower_service::Service;

mod messages;
mod request;
mod response;

pub use messages::{AdvanceStateMetadata, Output};
pub use request::Request;
pub use response::Response;

#[derive(Error, Debug)]
pub enum Error<E> {
    #[error("Reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("Deserialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("Deserialization hex parsing error: {0}")]
    HexParseError(#[from] hex::FromHexError),
    #[error("Cartesi Service Error: {0}")]
    ServiceError(E),
}

/// Repeatedly poll the given host for new requests
/// This works in standalone (no-backend) mode outside the Cartesi machine
/// and also works in the Cartesi machine when the http rollup interface is used
pub async fn listen_http<S>(service: &mut S, host_uri: &str) -> Result<(), Error<S::Error>>
where
    S: Service<Request, Response = Response>,
    S::Error: std::fmt::Debug,
{
    let client = reqwest::Client::new();

    let mut response = Response::empty_accept();
    loop {
        let resp = client
            .post(format!("{}/finish", host_uri))
            .json(&response.finish_message())
            .send()
            .await?;

        if resp.status() == reqwest::StatusCode::ACCEPTED {
            tracing::info!("No pending rollup request, trying again");
            continue; // no pending rollup request so run the loop again
        }
        let rollup_request: messages::RollupRequest = resp.json().await?;
        let request = Request::try_from(rollup_request)?;

        // let the dapp process the request
        match service.call(request).await.map_err(Error::ServiceError) {
            Ok(r) => {
                response = r;
                // handle the additional calls as required by the dApp outputs
                for output in response.outputs.iter() {
                    tracing::info!("Sending output {:?}", output);
                    let resp = client
                        .post(format!("{}/{}", host_uri, output.url_path()))
                        .json(&output)
                        .send()
                        .await;
                    tracing::info!("Output response: {:?}", resp);
                }
            }
            Err(e) => {
                tracing::error!("{:?}", e);
                response = Response::empty_reject();
            }
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
    response_derives = "Debug, PartialEq"
)]
pub struct InputsQuery;

/// Poll a graphql interface for new inputs
/// This will NOT work inside the Cartesi machine
/// It is indended to be used alongside a running machine to receive the same inputs
pub async fn listen_graphql<S>(
    service: &mut S,
    host_uri: &str,
    page_size: usize,
    frequency: std::time::Duration,
) -> Result<(), Error<S::Error>>
where
    S: Service<Request, Response = Response>,
    S::Error: std::fmt::Debug,
{
    let client = reqwest::Client::new();
    let mut cursor = None;

    let mut interval = interval(frequency);

    loop {
        let request_body = InputsQuery::build_query(inputs_query::Variables {
            first: page_size as i64,
            after: cursor.clone(),
        });
        let resp = client.post(host_uri).json(&request_body).send().await?;
        let response_body: graphql_client::Response<inputs_query::ResponseData> =
            resp.json().await?;
        for edge in response_body
            .data
            .unwrap()
            .inputs
            .edges
            .into_iter()
            .filter(|edge| edge.node.status == CompletionStatus::ACCEPTED)
        {
            cursor = Some(edge.cursor);
            match service
                .call(edge.node.try_into().unwrap())
                .await
                .map_err(Error::ServiceError)
            {
                Ok(r) => {
                    tracing::info!("Received response: {:?}", r);
                }
                Err(e) => {
                    tracing::error!("{:?}", e);
                }
            }
        }
        interval.tick().await;
    }
}
