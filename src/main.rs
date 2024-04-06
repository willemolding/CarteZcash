use service::{CarteZcashService, Request};
use std::env;
use std::error::Error;
use std::future::Future;
use std::pin::Pin;
use tower::{buffer::Buffer, util::BoxService, Service, ServiceExt};
use tower_cartesi::{CartesiRollApp, CartesiService};

use futures_util::future::FutureExt;

use zebra_chain::{block, parameters::Network};
use zebra_consensus::transaction as tx;

const DAPP_ADDRESS: &str = "70ac08179605AF2D9e75782b8DEcDD3c22aA4D0C";

mod service;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let subscriber = tracing_subscriber::fmt()
        .without_time()
        .with_max_level(tracing::Level::INFO)
        .compact()
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    print!(include_str!("ascii-logo.txt"));

    let network = Network::Mainnet;

    println!("Withdraw address is: {}", tiny_cash::mt_doom());

    let server_addr = env::var("ROLLUP_HTTP_SERVER_URL")?;

    let cartezcash_app = CarteZcashApp::new(network).await;
    let mut service = CartesiService::new(cartezcash_app);
    service.listen_http(&server_addr).await.unwrap();

    Ok(())
}

struct CarteZcashApp {
    cartezcash: Buffer<BoxService<Request, u64, Box<dyn Error + Sync + Send>>, Request>,
}

impl CarteZcashApp {
    pub async fn new(network: Network) -> Self {
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
    ) -> Pin<Box<dyn Future<Output = Result<tower_cartesi::Response, Box<dyn Error>>> + Send>> {
        let czk_request = Request::try_from((metadata, payload)).unwrap();
        let mut cartezcash_service = self.cartezcash.clone();
        async move {
            let burned = cartezcash_service
                .ready()
                .await
                .unwrap()
                .call(czk_request.clone())
                .await
                .map_err(|e| anyhow::anyhow!(e))
                .unwrap();

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
    ) -> Pin<Box<dyn Future<Output = Result<tower_cartesi::Response, Box<dyn Error>>> + Send>> {
        async move {
            tracing::info!("Received inspect state request");
            Ok(tower_cartesi::Response::empty_accept())
        }
        .boxed()
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

    // initialize the Halo2 verifier key
    // TODO: implement this

    Ok(())
}

fn withdraw_ether_call(receiver: ethereum_types::Address, value: ethereum_types::U256) -> Vec<u8> {
    let function = alloy_json_abi::Function::parse("withdrawEther(address,uint256)").unwrap();

    let encoded_params = ethabi::encode(&[
        ethabi::Token::Address(receiver),
        ethabi::Token::Uint(
            value
                .checked_mul(ethereum_types::U256::from(10_000_000_000_u64))
                .unwrap(),
        ),
    ]);

    let mut encoded = Vec::new();
    encoded.extend_from_slice(&function.selector().as_slice());
    encoded.extend_from_slice(&encoded_params);

    encoded
}
