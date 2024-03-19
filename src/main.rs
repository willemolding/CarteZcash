use json::{object, JsonValue};
use std::collections::HashMap;
use std::sync::Arc;
use std::env;
use tower::{service_fn, ServiceExt};
use chrono::{DateTime, TimeZone, Utc};

use zebra_chain::amount::{Amount, NonNegative};
use zebra_chain::transaction::{LockTime, Transaction};
use zebra_chain::parameters::{Network, NetworkUpgrade};
use zebra_chain::{block, transparent};
use zebra_chain::transaction::arbitrary::fake_v5_transactions_for_network;
use zebra_consensus::transaction::{Request, Verifier};
use zebra_test::mock_service::MockService;

pub async fn handle_advance(
    _client: &hyper::Client<hyper::client::HttpConnector>,
    _server_addr: &str,
    request: JsonValue,
) -> Result<&'static str, Box<dyn std::error::Error>> {
    println!("Received advance request data {}", &request);
    let _payload = request["data"]["payload"]
        .as_str()
        .ok_or("Missing payload")?;

    // run this here test
    println!("running test: mempool_request_with_present_input_is_accepted");
    test_verify_txn().await;
    println!("Test passed");

    Ok("accept")
}

pub async fn handle_inspect(
    _client: &hyper::Client<hyper::client::HttpConnector>,
    _server_addr: &str,
    request: JsonValue,
) -> Result<&'static str, Box<dyn std::error::Error>> {
    println!("Received inspect request data {}", &request);
    let _payload = request["data"]["payload"]
        .as_str()
        .ok_or("Missing payload")?;
    // TODO: add application logic here
    Ok("accept")
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = hyper::Client::new();
    let server_addr = env::var("ROLLUP_HTTP_SERVER_URL")?;

    let mut status = "accept";
    loop {
        println!("Sending finish");
        let response = object! {"status" => status.clone()};
        let request = hyper::Request::builder()
            .method(hyper::Method::POST)
            .header(hyper::header::CONTENT_TYPE, "application/json")
            .uri(format!("{}/finish", &server_addr))
            .body(hyper::Body::from(response.dump()))?;
        let response = client.request(request).await?;
        println!("Received finish status {}", response.status());

        if response.status() == hyper::StatusCode::ACCEPTED {
            println!("No pending rollup request, trying again");
        } else {
            let body = hyper::body::to_bytes(response).await?;
            let utf = std::str::from_utf8(&body)?;
            let req = json::parse(utf)?;

            let request_type = req["request_type"]
                .as_str()
                .ok_or("request_type is not a string")?;
            status = match request_type {
                "advance_state" => handle_advance(&client, &server_addr[..], req).await?,
                "inspect_state" => handle_inspect(&client, &server_addr[..], req).await?,
                &_ => {
                    eprintln!("Unknown request type");
                    "reject"
                }
            };
        }
    }
}

async fn test_verify_txn() {
    let network = Network::Mainnet;
    let nu5 = NetworkUpgrade::Nu5;
    let nu5_activation_height = nu5
        .activation_height(network)
        .expect("NU5 activation height is specified");

    let state_service = service_fn(|_| async { unreachable!("Service should not be called") });

    let verifier = Verifier::new(network, state_service);

    let mut transaction = fake_v5_transactions_for_network(network, zebra_test::vectors::MAINNET_BLOCKS.iter())
        .next_back()
        .expect("At least one fake V5 transaction in the test vectors");
    if transaction
        .expiry_height()
        .expect("V5 must have expiry_height")
        < nu5_activation_height
    {
        let expiry_height = transaction.expiry_height_mut();
        *expiry_height = nu5_activation_height;
    }

    let expected_hash = transaction.unmined_id();
    let expiry_height = transaction
        .expiry_height()
        .expect("V5 must have expiry_height");
    //
    let result = verifier
        .oneshot(Request::Block {
            transaction: Arc::new(transaction),
            known_utxos: Arc::new(HashMap::new()),
            height: expiry_height,
            time: DateTime::<Utc>::MAX_UTC,
        })
        .await;

    panic!("grr");

    assert_eq!(
        result.expect("unexpected error response").tx_id(),
        expected_hash
    );
}
