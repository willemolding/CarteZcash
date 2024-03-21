use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tower::buffer::Buffer;
use tower::{Service, ServiceExt};
use zebra_chain::block::Height;
use zebra_state::{HashOrHeight, ReadResponse};

use crate::proto::compact_formats::*;
use crate::proto::service::compact_tx_streamer_server::CompactTxStreamer;
use crate::proto::service::*;

#[derive(Clone)]
pub struct CompactTxStreamerImpl {
    pub state_read_service: Buffer<zebra_state::ReadStateService, zebra_state::ReadRequest>,
}

#[tonic::async_trait]
impl CompactTxStreamer for CompactTxStreamerImpl {
    /// Server streaming response type for the GetBlockRange method.
    type GetBlockRangeStream = ReceiverStream<Result<CompactBlock, tonic::Status>>;

    /// Server streaming response type for the GetTaddressTxids method.
    type GetTaddressTxidsStream = ReceiverStream<Result<RawTransaction, tonic::Status>>;

    /// Submit the given transaction to the Zcash network
    async fn send_transaction(
        &self,
        request: tonic::Request<RawTransaction>,
    ) -> std::result::Result<tonic::Response<SendResponse>, tonic::Status> {
        tracing::info!("send_transaction called");

        Err(tonic::Status::unimplemented(
            "gRPC endpoint not supported for cartezcash",
        ))
    }

    async fn get_latest_block(
        &self,
        request: tonic::Request<ChainSpec>,
    ) -> std::result::Result<tonic::Response<BlockId>, tonic::Status> {
        tracing::info!("get_latest_block called");

        let res: zebra_state::ReadResponse = self.state_read_service
            .clone()
            .ready()
            .await
            .unwrap()
            .call(zebra_state::ReadRequest::Tip)
            .await
            .unwrap();

        if let ReadResponse::Tip(Some((height, hash))) = res {
            tracing::info!("returning tip: {:?}", res);
            Ok(tonic::Response::new(BlockId {
                hash: hash.0.to_vec(),
                height: height.0 as u64,
            }))
        } else {
            tracing::info!("unexpected response");
            Err(tonic::Status::not_found(
                "Could not find the latest block in the state store",
            ))
        }
    }

    /// Return a list of consecutive compact blocks
    async fn get_block_range(
        &self,
        request: tonic::Request<BlockRange>,
    ) -> std::result::Result<tonic::Response<Self::GetBlockRangeStream>, tonic::Status> {
        tracing::info!("get_block_range called");
        let (tx, rx) = mpsc::channel(4);
        // todo: send the compact blocks into the tx end of the channel
        Ok(tonic::Response::new(ReceiverStream::new(rx)))
    }

    /// Return the requested full (not compact) transaction (as from zcashd)
    async fn get_transaction(
        &self,
        request: tonic::Request<TxFilter>,
    ) -> std::result::Result<tonic::Response<RawTransaction>, tonic::Status> {
        tracing::info!("get_transaction called");
        Ok(tonic::Response::new(RawTransaction {
            data: vec![],
            height: 0,
        }))
    }

    /// Return the txids corresponding to the given t-address within the given block range
    async fn get_taddress_txids(
        &self,
        request: tonic::Request<TransparentAddressBlockFilter>,
    ) -> std::result::Result<tonic::Response<Self::GetTaddressTxidsStream>, tonic::Status> {
        tracing::info!("get_taddress_txids called");
        let (tx, rx) = mpsc::channel(4);
        // TODO: Send the txiods into the tx end of the channel
        Ok(tonic::Response::new(ReceiverStream::new(rx)))
    }

    /// GetTreeState returns the note commitment tree state corresponding to the given block.
    /// See section 3.7 of the Zcash protocol specification. It returns several other useful
    /// values also (even though they can be obtained using GetBlock).
    /// The block can be specified by either height or hash.
    async fn get_tree_state(
        &self,
        request: tonic::Request<BlockId>,
    ) -> std::result::Result<tonic::Response<TreeState>, tonic::Status> {
        tracing::info!("get_tree_state called");

        let res: zebra_state::ReadResponse = self.state_read_service
            .clone()
            .ready()
            .await
            .unwrap()
            .call(zebra_state::ReadRequest::OrchardTree(HashOrHeight::Height(
                Height(request.into_inner().height.try_into().unwrap()),
            )))
            .await
            .unwrap();

        if let ReadResponse::OrchardTree(res) = res {
            tracing::info!("got orchard tree: {:?}", res);
        } else {
            tracing::info!("unexpected response");
        }

        let time = 0;
        let hash = String::new();
        let height = 0;

        // todo: do this properly
        let tree_state = TreeState {
            sapling_tree: String::new(),
            orchard_tree: String::new(),
            network: "cartezecash".to_string(),
            height,
            hash,
            time,
        };
        Ok(tonic::Response::new(tree_state))
    }

    /// Return information about this lightwalletd instance and the blockchain
    async fn get_lightd_info(
        &self,
        _request: tonic::Request<Empty>,
    ) -> std::result::Result<tonic::Response<LightdInfo>, tonic::Status> {
        tracing::info!("get_lightd_info called");

        let block_height = 0; // TODO: fetch this from the store

        let info = LightdInfo {
            version: env!("CARGO_PKG_VERSION").to_string(),
            vendor: "Wollum".to_string(),
            taddr_support: true,
            chain_name: "cartezcash".to_string(),
            sapling_activation_height: 0,
            consensus_branch_id: String::new(),
            block_height,
            git_commit: String::new(),
            branch: String::new(),
            build_date: String::new(),
            build_user: String::new(),
            estimated_height: block_height,
            zcashd_build: String::new(),
            zcashd_subversion: String::new(),
        };

        Ok(tonic::Response::new(info))
    }

    //////////////////////// The rest are just auto-generated stubs ////////////////////////

    /// Server streaming response type for the GetMempoolStream method.
    type GetMempoolStreamStream = ReceiverStream<Result<RawTransaction, tonic::Status>>;

    /// Return a stream of current Mempool transactions. This will keep the output stream open while
    /// there are mempool transactions. It will close the returned stream when a new block is mined.
    /// This does get called by zingo but it seems ok with receiving an error
    async fn get_mempool_stream(
        &self,
        request: tonic::Request<Empty>,
    ) -> std::result::Result<tonic::Response<Self::GetMempoolStreamStream>, tonic::Status> {
        tracing::info!("get_mempool_stream called. Ignoring request");
        // let (tx, rx) = mpsc::channel(4);
        // TODO: Send the txiods into the tx end of the channel
        Err(tonic::Status::unimplemented(
            "gRPC endpoint not supported for cartezcash",
        ))
        // Ok(tonic::Response::new(ReceiverStream::new(rx)))
    }

    /// Return the compact block corresponding to the given block identifier
    async fn get_block(
        &self,
        request: tonic::Request<BlockId>,
    ) -> std::result::Result<
        tonic::Response<crate::proto::compact_formats::CompactBlock>,
        tonic::Status,
    > {
        Err(tonic::Status::unimplemented(
            "gRPC endpoint not supported for cartezcash",
        ))
    }

    /// Same as GetBlock except actions contain only nullifiers
    async fn get_block_nullifiers(
        &self,
        request: tonic::Request<BlockId>,
    ) -> std::result::Result<
        tonic::Response<crate::proto::compact_formats::CompactBlock>,
        tonic::Status,
    > {
        Err(tonic::Status::unimplemented(
            "gRPC endpoint not supported for cartezcash",
        ))
    }

    /// Server streaming response type for the GetBlockRangeNullifiers method.
    type GetBlockRangeNullifiersStream = ReceiverStream<Result<CompactBlock, tonic::Status>>;

    /// Same as GetBlockRange except actions contain only nullifiers
    async fn get_block_range_nullifiers(
        &self,
        request: tonic::Request<BlockRange>,
    ) -> std::result::Result<tonic::Response<Self::GetBlockRangeNullifiersStream>, tonic::Status>
    {
        Err(tonic::Status::unimplemented(
            "gRPC endpoint not supported for cartezcash",
        ))
    }

    async fn get_taddress_balance(
        &self,
        request: tonic::Request<AddressList>,
    ) -> std::result::Result<tonic::Response<Balance>, tonic::Status> {
        Err(tonic::Status::unimplemented(
            "gRPC endpoint not supported for cartezcash",
        ))
    }

    async fn get_taddress_balance_stream(
        &self,
        request: tonic::Request<tonic::Streaming<Address>>,
    ) -> std::result::Result<tonic::Response<Balance>, tonic::Status> {
        Err(tonic::Status::unimplemented(
            "gRPC endpoint not supported for cartezcash",
        ))
    }

    /// Server streaming response type for the GetMempoolTx method.
    type GetMempoolTxStream = ReceiverStream<Result<CompactTx, tonic::Status>>;

    /// Return the compact transactions currently in the mempool; the results
    /// can be a few seconds out of date. If the Exclude list is empty, return
    /// all transactions; otherwise return all *except* those in the Exclude list
    /// (if any); this allows the client to avoid receiving transactions that it
    /// already has (from an earlier call to this rpc). The transaction IDs in the
    /// Exclude list can be shortened to any number of bytes to make the request
    /// more bandwidth-efficient; if two or more transactions in the mempool
    /// match a shortened txid, they are all sent (none is excluded). Transactions
    /// in the exclude list that don't exist in the mempool are ignored.
    async fn get_mempool_tx(
        &self,
        request: tonic::Request<Exclude>,
    ) -> std::result::Result<tonic::Response<Self::GetMempoolTxStream>, tonic::Status> {
        Err(tonic::Status::unimplemented(
            "gRPC endpoint not supported for cartezcash",
        ))
    }

    async fn get_latest_tree_state(
        &self,
        request: tonic::Request<Empty>,
    ) -> std::result::Result<tonic::Response<TreeState>, tonic::Status> {
        Err(tonic::Status::unimplemented(
            "gRPC endpoint not supported for cartezcash",
        ))
    }
    /// Server streaming response type for the GetSubtreeRoots method.
    type GetSubtreeRootsStream = ReceiverStream<Result<SubtreeRoot, tonic::Status>>;

    /// Returns a stream of information about roots of subtrees of the Sapling and Orchard
    /// note commitment trees.
    async fn get_subtree_roots(
        &self,
        request: tonic::Request<GetSubtreeRootsArg>,
    ) -> std::result::Result<tonic::Response<Self::GetSubtreeRootsStream>, tonic::Status> {
        Err(tonic::Status::unimplemented(
            "gRPC endpoint not supported for cartezcash",
        ))
    }
    async fn get_address_utxos(
        &self,
        request: tonic::Request<GetAddressUtxosArg>,
    ) -> std::result::Result<tonic::Response<GetAddressUtxosReplyList>, tonic::Status> {
        Err(tonic::Status::unimplemented(
            "gRPC endpoint not supported for cartezcash",
        ))
    }
    /// Server streaming response type for the GetAddressUtxosStream method.
    type GetAddressUtxosStreamStream = ReceiverStream<Result<GetAddressUtxosReply, tonic::Status>>;

    async fn get_address_utxos_stream(
        &self,
        request: tonic::Request<GetAddressUtxosArg>,
    ) -> std::result::Result<tonic::Response<Self::GetAddressUtxosStreamStream>, tonic::Status>
    {
        Err(tonic::Status::unimplemented(
            "gRPC endpoint not supported for cartezcash",
        ))
    }

    /// Testing-only, requires lightwalletd --ping-very-insecure (do not enable in production)
    async fn ping(
        &self,
        request: tonic::Request<Duration>,
    ) -> std::result::Result<tonic::Response<PingResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented(
            "gRPC endpoint not supported for cartezcash",
        ))
    }
}
