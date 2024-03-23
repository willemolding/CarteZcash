use std::env;
use service::{CarteZcashService, Request, Response};
use tower::{buffer::Buffer, util::BoxService, Service, ServiceExt};

use zebra_chain::{block, parameters::Network};
use zebra_consensus::transaction as tx;
use zebra_state;

mod service;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let subscriber = tracing_subscriber::FmtSubscriber::new();
    tracing::subscriber::set_global_default(subscriber)?;

    let network = Network::Mainnet;

    println!(
        "Withdraw address is: {}",
        tiny_cash::mt_doom().to_string()
    );

    let client = hyper::Client::new();
    let server_addr = env::var("ROLLUP_HTTP_SERVER_URL")?;

    // set up the services needed to run the rollup
    let (state_service, state_read_service, _, _) = zebra_state::init(
        zebra_state::Config::ephemeral(),
        network,
        block::Height::MAX,
        0,
    );
    let state_service = Buffer::new(state_service, 30);
    let verifier_service = tx::Verifier::new(network, state_service.clone());
    let mut tinycash = Buffer::new(
        BoxService::new(tiny_cash::write::TinyCashWriteService::new(
            state_service,
            verifier_service,
        )),
        10,
    );

    // mine the genesis block
    tinycash
        .ready()
        .await
        .unwrap()
        .call(tiny_cash::write::Request::Genesis)
        .await
        .unwrap();

    let mut cartezcash = BoxService::new(CarteZcashService::new(tinycash, state_read_service));

    let mut status = Response::Accept { burned: 0 };
    loop {
        println!("Sending finish");
        let response = client.request(status.host_request(&server_addr)).await?;

        if response.status() == hyper::StatusCode::ACCEPTED {
            println!("No pending rollup request, trying again");
        } else {
            let body = hyper::body::to_bytes(response).await?;
            let utf = std::str::from_utf8(&body)?;
            let req = json::parse(utf)?;
            println!("Received raw request: {:?}", req);
            let dapp_request = Request::try_from(req)?;
            println!("Parsed request: {:?}", dapp_request);

            status = cartezcash
                .call(dapp_request)
                .await
                .map_err(|e| anyhow::anyhow!(e))?;
            println!("Tinycash returned status: {:?}", &status);

            if let Some(report_request) = status.report_request(
                &server_addr,
            ) {
                println!("Sending report");
                let response = client.request(report_request).await?;
                println!(
                    "Received voucher status {}, {:?}",
                    response.status(),
                    hyper::body::to_bytes(response).await?
                );
            }
        }
    }
}
