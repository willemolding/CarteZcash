use service::{CarteZcashService, Request, Response};
use std::env;
use tower::{buffer::Buffer, util::BoxService, Service, ServiceExt};

use zebra_state;
use zebra_chain::{block, parameters::Network};
use zebra_consensus::transaction as tx;

mod service;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), anyhow::Error> {
    let subscriber = tracing_subscriber::FmtSubscriber::new();
    tracing::subscriber::set_global_default(subscriber)?;

    let client = hyper::Client::new();
    let server_addr = env::var("ROLLUP_HTTP_SERVER_URL")?;

    let network = Network::TinyCash;

    let (state_service, _, _, _) = zebra_state::init(
        zebra_state::Config::ephemeral(),
        network,
        block::Height::MAX,
        0,
    );
    let state_service = Buffer::new(state_service, 10);
    let verifier_service = tx::Verifier::new(network, state_service.clone());

    let mut tinycash =
        Buffer::new(BoxService::new(tiny_cash::write::TinyCashWriteService::new(state_service, verifier_service)), 10);

    tinycash
        .ready()
        .await
        .unwrap()
        .call(tiny_cash::write::Request::Genesis)
        .await
        .unwrap();

    let mut cartezcash =
        BoxService::new(CarteZcashService::new(tinycash));

    let mut status = Response::Accept { burned: 0 };
    loop {
        println!("Sending finish");
        let response = client.request(status.host_request(&server_addr)).await?;
        println!("Received finish status {}", response.status());

        if response.status() == hyper::StatusCode::ACCEPTED {
            println!("No pending rollup request, trying again");
        } else {

            let body = hyper::body::to_bytes(response).await?;
            let utf = std::str::from_utf8(&body)?;
            let req = json::parse(utf)?;
            println!("Received raw request: {:?}", req);
            let dapp_request = Request::try_from(req)?;
            println!("Parsed request: {:?}", dapp_request);

            status = cartezcash.call(dapp_request).await.unwrap();
            println!("Tinycash returned status: {:?}", &status);
        }
    }
}
