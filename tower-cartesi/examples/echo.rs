use futures_util::future::FutureExt;
use std::env;
use std::future::Future;
use std::pin::Pin;

use tower_cartesi::{BoxError, CartesiRollApp, CartesiService, Response};

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .compact()
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    let server_addr = env::var("ROLLUP_HTTP_SERVER_URL")?;

    let mut service = CartesiService::new(EchoApp {});
    service.listen_http(&server_addr).await?;

    Ok(())
}

struct EchoApp;

impl CartesiRollApp for EchoApp {
    fn handle_advance_state(
        &mut self,
        metadata: tower_cartesi::AdvanceStateMetadata,
        _payload: Vec<u8>,
    ) -> Pin<Box<dyn Future<Output = Result<Response, BoxError>> + Send>> {
        async move {
            tracing::info!("Received advance state request {:?}", metadata);
            Ok(tower_cartesi::Response::empty_accept())
        }
        .boxed()
    }

    fn handle_inspect_state(
        &mut self,
        _payload: Vec<u8>,
    ) -> Pin<Box<dyn Future<Output = Result<Response, BoxError>> + Send>> {
        async move {
            tracing::info!("Received inspect state request");
            Ok(tower_cartesi::Response::empty_accept())
        }
        .boxed()
    }
}
