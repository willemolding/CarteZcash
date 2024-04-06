use std::env;
use std::error::Error;

use tower_cartesi::{CartesiService, CartesiRollApp};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .compact()
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    let server_addr = env::var("ROLLUP_HTTP_SERVER_URL")?;

    let mut service = CartesiService::new(EchoApp{});
    service.listen_http(&server_addr).await?;

    Ok(())
}

struct EchoApp;

impl CartesiRollApp for EchoApp {
    fn handle_advance_state(
        &mut self,
        metadata: tower_cartesi::AdvanceStateMetadata,
        payload: Vec<u8>,
    ) -> impl std::future::Future<Output = Result<tower_cartesi::Response, Box<dyn Error>>> + Send + 'static {
        async move {
            println!("Received advance state request {:?}", metadata);
            Ok(tower_cartesi::Response::empty_accept())
        }
    }

    fn handle_inspect_state(
        &mut self,
        payload: Vec<u8>,
    ) -> impl std::future::Future<Output = Result<tower_cartesi::Response, Box<dyn Error>>> + Send + 'static {
        async move {
            println!("Received inspect state request");
            Ok(tower_cartesi::Response::empty_accept())
        }
    }
}
