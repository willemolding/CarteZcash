///
/// Runs a lightwalletd gRPC server that translates requests into HTTP requests to the /inspect API of a Cartesi machine running CarteZcash
/// Any ZCash wallet should be able to use this proxy to sync with the CarteZcash rollup
/// 

use crate::proto::service::compact_tx_streamer_server::CompactTxStreamerServer;
use tonic::transport::Server;
use tower::buffer::Buffer;

mod conversions;
mod inspect_state_read;
mod proto;
mod service_impl;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let subscriber = tracing_subscriber::FmtSubscriber::new();
    tracing::subscriber::set_global_default(subscriber)?;

    let addr = "[::1]:50051".parse()?;

    let state_read_service = inspect_state_read::InspectStateReader::new("0.0.0.0:8080".parse()?);
    let state_read_service = Buffer::new(state_read_service, 10);

    let svc =
        CompactTxStreamerServer::new(service_impl::CompactTxStreamerImpl { state_read_service });

    tracing::info!("Server listening on {}", addr);
    Server::builder()
        .trace_fn(|_| tracing::info_span!("cartezcash-proxy"))
        .add_service(svc)
        .serve(addr)
        .await?;
    Ok(())
}
