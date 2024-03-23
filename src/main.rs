use service::{CarteZcashService, Request, Response};
use std::{env, str::FromStr};
use tower::{buffer::Buffer, util::BoxService, Service, ServiceExt};

use zebra_chain::{amount::Amount, block, parameters::Network, serialization::ZcashDeserialize};
use zebra_consensus::transaction as tx;

mod service;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let subscriber = tracing_subscriber::FmtSubscriber::new();
    tracing::subscriber::set_global_default(subscriber)?;

    let network = Network::Mainnet;

    tracing::info!("Withdraw address is: {}", tiny_cash::mt_doom());

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

    initialize_network(&mut tinycash).await?;

    let mut cartezcash = BoxService::new(CarteZcashService::new(tinycash, state_read_service));

    let mut status = Response::Accept { burned: 0 };
    loop {
        tracing::info!("Sending finish");
        let response = client.request(status.host_request(&server_addr)).await?;

        if response.status() == hyper::StatusCode::ACCEPTED {
            tracing::info!("No pending rollup request, trying again");
        } else {
            let body = hyper::body::to_bytes(response).await?;
            let utf = std::str::from_utf8(&body)?;
            let req = json::parse(utf)?;
            let dapp_request = Request::try_from(req)?;

            tracing::info!("Received request: {:?}", dapp_request);

            status = cartezcash
                .call(dapp_request)
                .await
                .map_err(|e| anyhow::anyhow!(e))?;

            if status.report_request(&server_addr).is_none() {
                tracing::info!("CarteZcash responded with: {:?}", status);
            }

            if let Some(report_request) = status.report_request(&server_addr) {
                tracing::info!("Sending report");
                client.request(report_request).await?;
            }
        }
    }
}

async fn initialize_network<S>(tinycash: &mut S) -> Result<(), anyhow::Error>
where
    S: Service<
            tiny_cash::write::Request,
            Response = tiny_cash::write::Response,
            Error = tiny_cash::write::BoxError,
        > + Send
        + Clone
        + 'static,
    S::Future: Send + 'static,
{
    // Mine the genesis block
    tinycash
        .ready()
        .await
        .unwrap()
        .call(tiny_cash::write::Request::Genesis)
        .await
        .unwrap();

    Ok(())
}
