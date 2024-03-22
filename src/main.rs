use service::{CarteZcashService, Request, Response};
use std::env;
use tower::{buffer::Buffer, util::BoxService, Service, ServiceExt};
use zebra_chain::amount::Amount;
use zebra_chain::transparent::Script;

use zebra_chain::{block, parameters::Network};
use zebra_consensus::transaction as tx;
use zebra_state;

use cartezcash_proxy::proto::service::compact_tx_streamer_server::CompactTxStreamerServer;
use cartezcash_proxy::service_impl::CompactTxStreamerImpl;

mod service;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), anyhow::Error> {
    let subscriber = tracing_subscriber::FmtSubscriber::new();
    tracing::subscriber::set_global_default(subscriber)?;

    println!(
        "Withdraw address is: {}",
        tiny_cash::write::mt_doom().to_string()
    );

    let client = hyper::Client::new();
    let server_addr = env::var("ROLLUP_HTTP_SERVER_URL")?;

    let network = Network::Mainnet;

    let (state_service, state_read_service, _, _) = zebra_state::init(
        zebra_state::Config::ephemeral(),
        network,
        block::Height::MAX,
        0,
    );
    let state_service = Buffer::new(state_service, 30);
    let verifier_service = tx::Verifier::new(network, state_service.clone());

    // run the proxy here
    // TODO: Put this behind a feature flag, it is for testing only

    let state_read_service = Buffer::new(state_read_service, 10);
    let svc = CompactTxStreamerServer::new(CompactTxStreamerImpl { state_read_service });

    let addr = "[::1]:50051".parse()?;
    println!("Server listening on {}", addr);

    let grpc_server = tonic::transport::Server::builder()
        .trace_fn(|_| tracing::info_span!("cartezcash-proxy"))
        .add_service(svc)
        .serve(addr);
    tokio::spawn(grpc_server);

    let mut tinycash = Buffer::new(
        BoxService::new(tiny_cash::write::TinyCashWriteService::new(
            state_service,
            verifier_service,
        )),
        10,
    );

    tinycash
        .ready()
        .await
        .unwrap()
        .call(tiny_cash::write::Request::Genesis)
        .await
        .unwrap();

    let mut cartezcash = BoxService::new(CarteZcashService::new(tinycash));

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

            if let Some(voucher_request) = status.voucher_request(
                &server_addr,
                ethereum_types::Address::random(),
                ethereum_types::U256::from(888),
            ) {
                println!("Sending voucher");
                let response = client.request(voucher_request).await?;
                println!(
                    "Received voucher status {}, {:?}",
                    response.status(),
                    hyper::body::to_bytes(response).await?
                );
            }
        }
    }
}
