use crate::proto::service::compact_tx_streamer_server::CompactTxStreamerServer;
use tonic::transport::Server;
use tower::buffer::Buffer;
use zebra_chain::block;
use zebra_chain::parameters::Network;

mod proto;
mod service_impl;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let subscriber = tracing_subscriber::FmtSubscriber::new();
    tracing::subscriber::set_global_default(subscriber)?;

    let addr = "[::1]:50051".parse()?;

    let (_, state_read_service, _, _) = zebra_state::init(
        zebra_state::Config::ephemeral(),
        Network::Testnet,
        block::Height::MAX,
        0,
    );
    let state_read_service = Buffer::new(state_read_service, 10);

    let svc = CompactTxStreamerServer::new(service_impl::CompactTxStreamerImpl { state_read_service });

    println!("Server listening on {}", addr);
    Server::builder()
        .trace_fn(|_| tracing::info_span!("cartezcash-proxy"))
        .add_service(svc)
        .serve(addr)
        .await?;
    Ok(())
}
