use cartezcash_lightwalletd::{
    proto::service::compact_tx_streamer_server::CompactTxStreamerServer,
    service_impl::CompactTxStreamerImpl,
};
use service::{CarteZcashService, Request};
use std::env;
use std::error::Error;
use std::future::Future;
use std::pin::Pin;
use tower::{buffer::Buffer, util::BoxService, Service, ServiceExt};
use tower_cartesi::{BoxError, CartesiRollApp, CartesiService};

use futures_util::future::FutureExt;

use zebra_chain::{block, parameters::Network};
use zebra_consensus::transaction as tx;

type StateService = Buffer<BoxService<zebra_state::Request, zebra_state::Response, zebra_state::BoxError>, zebra_state::Request>;
type StateReadService = Buffer<BoxService<zebra_state::ReadRequest, zebra_state::ReadResponse, zebra_state::BoxError>, zebra_state::ReadRequest>;

mod service;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let subscriber = tracing_subscriber::fmt()
        .without_time()
        .with_max_level(tracing::Level::INFO)
        .compact()
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    let server_addr = env::var("ROLLUP_HTTP_SERVER_URL")?;
    let grpc_addr = env::var("GRPC_SERVER_URL")?;

    let network = Network::Mainnet;

    println!("Withdraw address is: {}", tiny_cash::mt_doom());

    tracing::info!("Initializing Halo2 verifier key");
    tiny_cash::initialize_halo2();
    tracing::info!("Initializing Halo2 verifier key complete");

    let (state_service, state_read_service, _, _) = zebra_state::init(
        zebra_state::Config::ephemeral(),
        network,
        block::Height::MAX,
        0,
    );
    let state_service = Buffer::new(state_service, 30);
    let state_read_service = Buffer::new(state_read_service.boxed(), 30);

    let cartezcash_app = CarteZcashApp::new(network, state_service, state_read_service.clone()).await;

    let svc = CompactTxStreamerServer::new(CompactTxStreamerImpl { state_read_service });
    let addr = grpc_addr.parse()?;
    let grpc_server = tonic::transport::Server::builder()
        .trace_fn(|_| tracing::info_span!("cartezcash-grpc"))
        .add_service(svc)
        .serve(addr);
    tokio::spawn(grpc_server);
    tracing::info!("wallet GRPC server listening on {}", addr);

    let mut service = CartesiService::new(cartezcash_app);
    service.listen_http(&server_addr).await.expect("Failed to start the rollup server");

    Ok(())
}

struct CarteZcashApp {
    cartezcash: Buffer<BoxService<Request, u64, Box<dyn Error + Sync + Send>>, Request>,
}

impl CarteZcashApp {
    pub async fn new(
        network: Network,
        state_service: StateService,
        state_read_service: StateReadService,
    ) -> Self {
        // set up the services needed to run the rollup
        let verifier_service = tx::Verifier::new(network, state_service.clone());
        let mut tinycash = Buffer::new(
            BoxService::new(tiny_cash::write::TinyCashWriteService::new(
                state_service,
                verifier_service,
            )),
            10,
        );

        initialize_network(&mut tinycash).await.unwrap();

        Self {
            cartezcash: Buffer::new(
                BoxService::new(CarteZcashService::new(tinycash, state_read_service)),
                10,
            ),
        }
    }
}

impl CartesiRollApp for CarteZcashApp {
    fn handle_advance_state(
        &mut self,
        metadata: tower_cartesi::AdvanceStateMetadata,
        payload: Vec<u8>,
    ) -> Pin<Box<dyn Future<Output = Result<tower_cartesi::Response, BoxError>> + Send>> {
        let czk_request = Request::try_from((metadata, payload)).unwrap();
        let mut cartezcash_service = self.cartezcash.clone();
        async move {
            let burned = cartezcash_service
                .ready()
                .await?
                .call(czk_request.clone())
                .await?;

            let response = tower_cartesi::Response::empty_accept();
            if burned > 0 {
                // add the voucher here
            }

            Ok(response)
        }
        .boxed()
    }

    fn handle_inspect_state(
        &mut self,
        payload: Vec<u8>,
    ) -> Pin<Box<dyn Future<Output = Result<tower_cartesi::Response, BoxError>> + Send>> {
        async move {
            tracing::info!("Received inspect state request. Ignoring.");
            Ok(tower_cartesi::Response::empty_accept())
        }
        .boxed()
    }
}

async fn initialize_network<S>(tinycash: &mut S) -> Result<(), BoxError>
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
        .await?
        .call(tiny_cash::write::Request::Genesis)
        .await?;
    Ok(())
}
