use service::{CarteZcashService, Request};
use zcash_keys::address::UnifiedAddress;
use zcash_primitives::consensus::MAIN_NETWORK;

use std::env;
use std::error::Error;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tower::{buffer::Buffer, util::BoxService, BoxError, Service, ServiceExt};
use tower_cartesi::{listen_http, Request as RollAppRequest, Response};

use futures_util::future::FutureExt;

use zebra_chain::{block, parameters::Network};
use zebra_consensus::transaction as tx;

#[cfg(feature = "lightwalletd")]
use cartezcash_lightwalletd::{
    proto::service::compact_tx_streamer_server::CompactTxStreamerServer,
    service_impl::CompactTxStreamerImpl,
};

type StateService = Buffer<
    BoxService<zebra_state::Request, zebra_state::Response, zebra_state::BoxError>,
    zebra_state::Request,
>;
type StateReadService = Buffer<
    BoxService<zebra_state::ReadRequest, zebra_state::ReadResponse, zebra_state::BoxError>,
    zebra_state::ReadRequest,
>;

mod service;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .compact()
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    let server_addr = env::var("ROLLUP_HTTP_SERVER_URL")?;

    let network = Network::Mainnet;

    println!(
        "Withdraw address is: {}",
        UnifiedAddress::from_receivers(Some(tiny_cash::mt_doom_address()), None)
            .unwrap()
            .encode(&MAIN_NETWORK)
    );

    // TODO: Enable this when not debugging
    // tracing::info!("Initializing Halo2 verifier key");
    // tiny_cash::initialize_halo2();
    // tracing::info!("Initializing Halo2 verifier key complete");

    let (state_service, state_read_service, _, _) = zebra_state::init(
        zebra_state::Config::ephemeral(),
        network,
        block::Height::MAX,
        0,
    );
    let state_service = Buffer::new(state_service, 30);

    let mut cartezcash_app = CarteZcashApp::new(state_service).await;

    #[cfg(feature = "lightwalletd")]
    {
        let grpc_addr = env::var("GRPC_SERVER_URL")?;
        let state_read_service = Buffer::new(state_read_service.boxed(), 30);
        let svc = CompactTxStreamerServer::new(CompactTxStreamerImpl { state_read_service });
        let addr = grpc_addr.parse()?;
        let grpc_server = tonic::transport::Server::builder()
            .trace_fn(|_| tracing::info_span!("cartezcash-grpc"))
            .add_service(svc)
            .serve(addr);
        tokio::spawn(grpc_server);
        tracing::info!("wallet GRPC server listening on {}", addr);
    }

    listen_http(&mut cartezcash_app, &server_addr)
        .await
        .expect("Failed to start the rollup server");

    Ok(())
}

struct CarteZcashApp {
    cartezcash:
        Buffer<BoxService<Request, service::Response, Box<dyn Error + Sync + Send>>, Request>,
}

impl CarteZcashApp {
    pub async fn new(state_service: StateService) -> Self {
        // set up the services needed to run the rollup
        let mut tinycash = Buffer::new(
            BoxService::new(tiny_cash::write::TinyCashWriteService::new(
                state_service,
            )),
            10,
        );

        initialize_network(&mut tinycash).await.unwrap();

        Self {
            cartezcash: Buffer::new(BoxService::new(CarteZcashService::new(tinycash)), 10),
        }
    }
}

impl Service<RollAppRequest> for CarteZcashApp {
    type Response = Response;
    type Error = Box<dyn Error + Send + Sync>;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: RollAppRequest) -> Self::Future {
        match req {
            RollAppRequest::AdvanceState { metadata, payload } => {
                let czk_request = Request::try_from((metadata, payload)).unwrap();
                let mut cartezcash_service = self.cartezcash.clone();
                async move {
                    let response = cartezcash_service
                        .ready()
                        .await?
                        .call(czk_request.clone())
                        .await?;

                    let resp = tower_cartesi::Response::empty_accept();
                    for withdrawal in response.withdrawals {
                        // TODO: Build vouchers and add to response
                        tracing::info!("Withdrawal: {:?}", withdrawal);
                    }
                    Ok(resp)
                }
                .boxed()
            }
            RollAppRequest::InspectState { payload } => {
                println!("Received inspect state request {:?}", payload);
                async { Ok(tower_cartesi::Response::empty_accept()) }.boxed()
            }
        }
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
