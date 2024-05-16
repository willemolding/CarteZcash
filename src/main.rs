use service::{CarteZcashService, Request};
use zcash_keys::address::UnifiedAddress;
use zcash_primitives::consensus::MAIN_NETWORK;

use std::env;
use std::error::Error;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tower::{buffer::Buffer, util::BoxService, BoxError, Service, ServiceExt};
use tower_cartesi::{Request as RollAppRequest, Response};

use futures_util::future::FutureExt;

#[cfg(feature = "lightwalletd")]
use cartezcash_lightwalletd::{
    proto::service::compact_tx_streamer_server::CompactTxStreamerServer,
    service_impl::CompactTxStreamerImpl,
};

#[cfg(feature = "lightwalletd")]
type StateService = Buffer<
    BoxService<zebra_state::Request, zebra_state::Response, zebra_state::BoxError>,
    zebra_state::Request,
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

    println!(
        "Withdraw address is: {}",
        UnifiedAddress::from_receivers(Some(tiny_cash::mt_doom_address()), None)
            .unwrap()
            .encode(&MAIN_NETWORK)
    );

    // TODO: Enable this when not debugging
    #[cfg(feature = "preinitialize-halo2")]
    {
        tracing::info!("Initializing Halo2 verifier key");
        tiny_cash::initialize_halo2();
        tracing::info!("Initializing Halo2 verifier key complete");
    }

    #[cfg(feature = "lightwalletd")]
    let (state_service, state_read_service, _, _) = zebra_state::init(
        zebra_state::Config::ephemeral(),
        tiny_cash::parameters::Network::Mainnet,
        tiny_cash::block::Height::MAX,
        0,
    );

    let mut cartezcash_app = CarteZcashApp::new(
        #[cfg(feature = "lightwalletd")]
        Buffer::new(state_service, 30),
    )
    .await;

    #[cfg(feature = "lightwalletd")]
    {
        let grpc_addr = env::var("GRPC_SERVER_URL")?;
        let state_read_service = Buffer::new(state_read_service.boxed(), 30);
        let svc = CompactTxStreamerServer::new(CompactTxStreamerImpl::new( 
            state_read_service,
            env::var("ETH_RPC_URL")?,
            env::var("ETH_CHAIN_ID").map(|s| s.parse::<u64>().unwrap())?,
            env::var("SIGNER_PK")?,
            env::var("INPUTBOX_CONTRACT_ADDRESS")?,
            env::var("DAPP_ADDRESS")?
        ));
        let addr = grpc_addr.parse()?;
        let grpc_server = tonic::transport::Server::builder()
            .trace_fn(|_| tracing::info_span!("cartezcash-grpc"))
            .add_service(svc)
            .serve(addr);
        tokio::spawn(grpc_server);
        tracing::info!("wallet GRPC server listening on {}", addr);
    }

    #[cfg(feature = "listen-http")]
    tower_cartesi::listen_http(&mut cartezcash_app, &server_addr)
        .await
        .expect("Failed to start the rollup server");

    #[cfg(feature = "listen-graphql")]
    tower_cartesi::listen_graphql(
        &mut cartezcash_app,
        &server_addr,
        10,
        std::time::Duration::from_secs(5),
    )
    .await
    .expect("Failed to start the rollup server");

    Ok(())
}

struct CarteZcashApp {
    cartezcash:
        Buffer<BoxService<Request, service::Response, Box<dyn Error + Sync + Send>>, Request>,
    #[cfg(feature = "lightwalletd")]
    state_service: StateService,
    dapp_address: Option<ethereum_types::Address>,
}

impl CarteZcashApp {
    pub async fn new(#[cfg(feature = "lightwalletd")] mut state_service: StateService) -> Self {
        // set up the services needed to run the rollup
        let mut tinycash = Buffer::new(BoxService::new(tiny_cash::service::TinyCash::new()), 10);

        initialize_network(
            &mut tinycash,
            #[cfg(feature = "lightwalletd")]
            &mut state_service,
        )
        .await
        .unwrap();

        Self {
            cartezcash: Buffer::new(BoxService::new(CarteZcashService::new(tinycash)), 10),
            #[cfg(feature = "lightwalletd")]
            state_service: state_service,
            dapp_address: None,
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
                // if the payload is 20 bytes, it is the dapp address. Handle it then exit early
                if payload.len() == 20 {
                    let dapp_address = ethereum_types::Address::from_slice(&payload);
                    tracing::info!("Received dapp address: {:?}", dapp_address);
                    self.dapp_address = Some(dapp_address);
                    return async { Ok(tower_cartesi::Response::empty_accept()) }.boxed();
                }

                let mut cartezcash_service = self.cartezcash.clone();

                #[cfg(feature = "lightwalletd")]
                let mut state_service = self.state_service.clone();
                let dapp_address = self.dapp_address.clone();
                async move {
                    let czk_request = Request::try_from((metadata, payload))?;

                    let response = cartezcash_service
                        .ready()
                        .await?
                        .call(czk_request.clone())
                        .await?;

                    #[cfg(feature = "lightwalletd")]
                    {
                        tracing::info!(
                            "committing block {} at height {:?}",
                            response.block.hash,
                            response.block.height
                        );
                        state_service
                            .ready()
                            .await?
                            .call(zebra_state::Request::CommitSemanticallyVerifiedBlock(
                                response.block,
                            ))
                            .await?;
                    }
                    let mut resp = tower_cartesi::Response::empty_accept();
                    for (recipient, amount) in response.withdrawals {
                        tracing::info!("Withdrawal: to {:?} with amount: {:?}", recipient, amount);
                        if let Some(dapp_address) = dapp_address {
                            resp.add_voucher(dapp_address, &encode_withdraw_call(recipient, amount));
                        } else {
                            tracing::error!("Withdrawal made before dapp address set. Funds are lost.");
                        }
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

async fn initialize_network<S>(
    tinycash: &mut S,
    #[cfg(feature = "lightwalletd")] state_service: &mut StateService,
) -> Result<(), BoxError>
where
    S: Service<
            tiny_cash::service::Request,
            Response = tiny_cash::service::Response,
            Error = BoxError,
        > + Send
        + Clone
        + 'static,
    S::Future: Send + 'static,
{
    // Mine the genesis block
    let response = tinycash
        .ready()
        .await?
        .call(tiny_cash::service::Request::Genesis)
        .await?;

    tracing::info!(
        "committing GENESIS block {} at height {:?}",
        response.block.hash,
        response.block.height
    );

    #[cfg(feature = "lightwalletd")]
    state_service
        .ready()
        .await?
        .call(zebra_state::Request::CommitCheckpointVerifiedBlock(
            zebra_state::CheckpointVerifiedBlock::from(response.block.block),
        ))
        .await?;

    Ok(())
}

fn encode_withdraw_call(recipient: ethereum_types::Address, amount: ethereum_types::U256) -> Vec<u8> {
    let function = alloy_json_abi::Function::parse("withdrawEther(address,uint256)").unwrap();
    let encoded_params = ethabi::encode(&[ethabi::Token::Address(recipient), ethabi::Token::Uint(amount)]);
    let mut encoded = Vec::new();
    encoded.extend_from_slice(&function.selector().as_slice());
    encoded.extend_from_slice(&encoded_params);
    encoded
}