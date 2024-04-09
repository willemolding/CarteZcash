use futures_util::future::TryFutureExt;
use std::collections::HashSet;
use std::str::FromStr;

use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tower::{Service, ServiceExt};
use zebra_chain::block::Height;
use zebra_chain::transparent;
use zebra_state::{HashOrHeight, IntoDisk, ReadResponse};

use crate::conversions;
use crate::proto::compact_formats::*;
use crate::proto::service::compact_tx_streamer_server::CompactTxStreamer;
use crate::proto::service::*;

use ethers::middleware::SignerMiddleware;
use ethers::prelude::abigen;
use ethers::providers::{Http, Provider};
use ethers::signers::{LocalWallet, Signer};
use ethers::types::{Address, Bytes};

abigen!(
    IInputBox,
    "[function addInput(address appContract, bytes calldata payload) external returns (bytes32)]"
);

#[derive(Clone)]
pub struct CompactTxStreamerImpl<R> {
    pub state_read_service: R, //Buffer<zebra_state::ReadStateService, zebra_state::ReadRequest>,
}

#[tonic::async_trait]
impl<R> CompactTxStreamer for CompactTxStreamerImpl<R>
where
    R: Service<
            zebra_state::ReadRequest,
            Response = zebra_state::ReadResponse,
            Error = zebra_state::BoxError,
        > + Send
        + Sync
        + Clone
        + 'static,
    R::Future: Send + 'static,
{
    /// Server streaming response type for the GetBlockRange method.
    type GetBlockRangeStream = ReceiverStream<Result<CompactBlock, tonic::Status>>;

    /// Server streaming response type for the GetTaddressTxids method.
    type GetTaddressTxidsStream = ReceiverStream<Result<RawTransaction, tonic::Status>>;

    /// Submit the given transaction to the Zcash network
    /// TODO: This is a hacky implementatin to speed up tests. Write a better one in the future
    async fn send_transaction(
        &self,
        request: tonic::Request<RawTransaction>,
    ) -> std::result::Result<tonic::Response<SendResponse>, tonic::Status> {
        tracing::info!("send_transaction called. Fowarding to InputBox contract");

        let provider = Provider::<Http>::try_from("http://127.0.0.1:8545").unwrap();
        let wallet: LocalWallet =
            "ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
                .parse()
                .unwrap();
        let client = std::sync::Arc::new(SignerMiddleware::new(
            provider,
            wallet.with_chain_id(31337_u64),
        ));

        // Instantiate the contract
        let contract = IInputBox::new(
            Address::from_str("0x59b22D57D4f067708AB0c00552767405926dc768").unwrap(),
            client,
        );
        contract
            .add_input(
                Address::from_str("0x70ac08179605AF2D9e75782b8DEcDD3c22aA4D0C").unwrap(),
                Bytes::from(request.get_ref().data.clone()),
            )
            .send()
            .await
            .unwrap();

        Ok(tonic::Response::new(SendResponse {
            error_code: 0,
            error_message: "".to_string(),
        }))
    }

    async fn get_latest_block(
        &self,
        _request: tonic::Request<ChainSpec>,
    ) -> std::result::Result<tonic::Response<BlockId>, tonic::Status> {
        tracing::info!("get_latest_block called");

        let res: zebra_state::ReadResponse = self
            .state_read_service
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
        tracing::info!("get_block_range called with: {:?} ", request);
        let (tx, rx) = mpsc::channel(10);

        // these sometimes come in reverse order...
        let block_range = request.into_inner();
        let a = block_range.start.unwrap().height;
        let b = block_range.end.unwrap().height;

        let start = std::cmp::min(a, b);
        let end = std::cmp::max(a, b);
        let range = if a < b {
            (start..=end).collect::<Vec<u64>>()
        } else {
            (start..=end).rev().collect()
        };

        let mut state_read_service = self.state_read_service.clone();

        for height in range {
            tracing::info!("fetching block at height: {}", height);
            let res: zebra_state::ReadResponse = state_read_service
                .ready()
                .await
                .unwrap()
                .call(zebra_state::ReadRequest::Block(HashOrHeight::Height(
                    Height(height.try_into().unwrap()),
                )))
                .await
                .unwrap();

            let block = match res {
                ReadResponse::Block(Some(block)) => block,
                _ => {
                    tracing::info!("unexpected response");
                    return Err(tonic::Status::not_found(
                        "Could not find the block in the state store",
                    ));
                }
            };
            let hash = block.hash();

            // Sapling trees
            //
            // # Concurrency
            //
            // We look up by block hash so the hash, transaction IDs, and confirmations
            // are consistent.
            let request = zebra_state::ReadRequest::SaplingTree(hash.into());
            let response = state_read_service
                .ready()
                .and_then(|service| service.call(request))
                .await
                .unwrap();

            let sapling_commitment_tree_size = match response {
                zebra_state::ReadResponse::SaplingTree(Some(nct)) => nct.count(),
                zebra_state::ReadResponse::SaplingTree(None) => 0,
                _ => unreachable!("unmatched response to a SaplingTree request"),
            } as u32;

            // Orchard trees
            //
            // # Concurrency
            //
            // We look up by block hash so the hash, transaction IDs, and confirmations
            // are consistent.
            let request = zebra_state::ReadRequest::OrchardTree(hash.into());
            let response = state_read_service
                .ready()
                .and_then(|service| service.call(request))
                .await
                .unwrap();

            let orchard_commitment_tree_size = match response {
                zebra_state::ReadResponse::OrchardTree(Some(nct)) => nct.count(),
                zebra_state::ReadResponse::OrchardTree(None) => 0,
                _ => unreachable!("unmatched response to a OrchardTree request"),
            } as u32;

            let compact_block = conversions::block_to_compact(
                &block,
                ChainMetadata {
                    sapling_commitment_tree_size,
                    orchard_commitment_tree_size,
                },
            );
            tracing::debug!("sending block: {:?}", compact_block);
            tx.send(Ok(compact_block)).await.unwrap();
        }

        Ok(tonic::Response::new(ReceiverStream::new(rx)))
    }

    /// Return the requested full (not compact) transaction (as from zcashd)
    async fn get_transaction(
        &self,
        request: tonic::Request<TxFilter>,
    ) -> std::result::Result<tonic::Response<RawTransaction>, tonic::Status> {
        tracing::info!("get_transaction called");
        let mut state_read_service = self.state_read_service.clone();

        let request = zebra_state::ReadRequest::Transaction(
            zebra_chain::transaction::Hash::from_bytes_in_display_order(
                &request.into_inner().hash.try_into().unwrap(),
            ),
        );
        let response = state_read_service
            .ready()
            .and_then(|service| service.call(request))
            .await
            .unwrap();
        if let zebra_state::ReadResponse::Transaction(Some(transaction)) = response {
            Ok(tonic::Response::new(RawTransaction {
                data: transaction.tx.as_bytes(),
                height: transaction.height.0 as u64,
            }))
        } else {
            tracing::info!("unexpected response");
            Err(tonic::Status::not_found(
                "Could not find the transaction in the state store",
            ))
        }
    }

    /// Return the txids corresponding to the given t-address within the given block range
    async fn get_taddress_txids(
        &self,
        request: tonic::Request<TransparentAddressBlockFilter>,
    ) -> std::result::Result<tonic::Response<Self::GetTaddressTxidsStream>, tonic::Status> {
        tracing::info!("get_taddress_txids called with {:?}", request);

        let request = request.into_inner();
        let address = transparent::Address::from_str(&request.address).unwrap();

        let mut addresses = HashSet::new();
        addresses.insert(address);

        let block_range = request.range.unwrap();

        let mut state_read_service = self.state_read_service.clone();

        let res: zebra_state::ReadResponse = state_read_service
            .ready()
            .await
            .unwrap()
            .call(zebra_state::ReadRequest::TransactionIdsByAddresses {
                addresses,
                height_range: Height(block_range.start.unwrap().height as u32)
                    ..=Height(block_range.end.unwrap().height as u32),
            })
            .await
            .unwrap();

        if let ReadResponse::AddressesTransactionIds(txns) = res {
            let (tx, rx) = mpsc::channel(10);
            tracing::info!("{:?} transactions found", txns.len());
            for (_location, tx_id) in txns.iter() {
                tracing::debug!("got txid: {:?}", tx_id);

                let res = state_read_service
                    .ready()
                    .await
                    .unwrap()
                    .call(zebra_state::ReadRequest::Transaction(*tx_id))
                    .await
                    .unwrap();

                if let ReadResponse::Transaction(Some(transaction)) = res {
                    tx.send(Ok(RawTransaction {
                        data: transaction.tx.as_bytes(),
                        height: transaction.height.0 as u64,
                    }))
                    .await
                    .unwrap();
                } else {
                    tracing::debug!("unexpected response");
                }
            }
            Ok(tonic::Response::new(ReceiverStream::new(rx)))
        } else {
            tracing::debug!("unexpected response");
            Err(tonic::Status::unimplemented(
                "unexpcted response from TransactionIdsByAddresses",
            ))
        }
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

        let mut read_service = self.state_read_service.clone();
        let height: Height = Height(request.into_inner().height.try_into().unwrap());

        let res: zebra_state::ReadResponse = read_service
            .ready()
            .await
            .unwrap()
            .call(zebra_state::ReadRequest::Block(HashOrHeight::Height(
                height,
            )))
            .await
            .unwrap();

        let block = match res {
            ReadResponse::Block(Some(block)) => block,
            _ => {
                tracing::debug!("unexpected response");
                return Err(tonic::Status::not_found(
                    "Could not find the block in the state store",
                ));
            }
        };
        let hash = block.hash();

        // the following taken and modified from https://github.com/ZcashFoundation/zebra/blob/f79fc6aa8eff0db98e8eae53194325188ee96915/zebra-rpc/src/methods.rs#L1102

        let hash_or_height = HashOrHeight::Height(height);

        let orchard_request = zebra_state::ReadRequest::OrchardTree(hash_or_height);
        let orchard_response = read_service
            .ready()
            .and_then(|service| service.call(orchard_request))
            .await
            .unwrap();

        let orchard_tree_hex = match orchard_response {
            zebra_state::ReadResponse::OrchardTree(maybe_tree) => {
                let tree = zebra_chain::orchard::tree::SerializedTree::from(maybe_tree);
                hex::encode(tree)
            }
            _ => unreachable!("unmatched response to an orchard tree request"),
        };

        let tree_state = TreeState {
            sapling_tree: hex::encode([0u8; 413]), // Sapling not supported but this stops the wallets from crashing
            orchard_tree: orchard_tree_hex,
            network: "mainnet".to_string(),
            height: height.0 as u64,
            hash: hash.to_string(),
            time: block.header.time.timestamp() as u32,
        };
        tracing::info!("returning tree state: {:?}", tree_state);
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
            chain_name: "mainnet".to_string(),
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
        _request: tonic::Request<Empty>,
    ) -> std::result::Result<tonic::Response<Self::GetMempoolStreamStream>, tonic::Status> {
        tracing::info!("get_mempool_stream called");
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
        _request: tonic::Request<BlockId>,
    ) -> std::result::Result<
        tonic::Response<crate::proto::compact_formats::CompactBlock>,
        tonic::Status,
    > {
        tracing::info!("get_block called. Ignoring request");
        Err(tonic::Status::unimplemented(
            "gRPC endpoint not supported for cartezcash",
        ))
    }

    /// Same as GetBlock except actions contain only nullifiers
    async fn get_block_nullifiers(
        &self,
        _request: tonic::Request<BlockId>,
    ) -> std::result::Result<
        tonic::Response<crate::proto::compact_formats::CompactBlock>,
        tonic::Status,
    > {
        tracing::info!("get_block_nullifiers called. Ignoring request");
        Err(tonic::Status::unimplemented(
            "gRPC endpoint not supported for cartezcash",
        ))
    }

    /// Server streaming response type for the GetBlockRangeNullifiers method.
    type GetBlockRangeNullifiersStream = ReceiverStream<Result<CompactBlock, tonic::Status>>;

    /// Same as GetBlockRange except actions contain only nullifiers
    async fn get_block_range_nullifiers(
        &self,
        _request: tonic::Request<BlockRange>,
    ) -> std::result::Result<tonic::Response<Self::GetBlockRangeNullifiersStream>, tonic::Status>
    {
        tracing::info!("get_block_range_nullifiers called. Ignoring request");
        Err(tonic::Status::unimplemented(
            "gRPC endpoint not supported for cartezcash",
        ))
    }

    async fn get_taddress_balance(
        &self,
        _request: tonic::Request<AddressList>,
    ) -> std::result::Result<tonic::Response<Balance>, tonic::Status> {
        tracing::info!("get_taddress_balance called. Ignoring request");
        Err(tonic::Status::unimplemented(
            "gRPC endpoint not supported for cartezcash",
        ))
    }

    async fn get_taddress_balance_stream(
        &self,
        _request: tonic::Request<tonic::Streaming<crate::proto::service::Address>>,
    ) -> std::result::Result<tonic::Response<Balance>, tonic::Status> {
        tracing::info!("get_taddress_balance_stream called. Ignoring request");
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
        _request: tonic::Request<Exclude>,
    ) -> std::result::Result<tonic::Response<Self::GetMempoolTxStream>, tonic::Status> {
        tracing::info!("get_mempool_tx called. Ignoring request");
        Err(tonic::Status::unimplemented(
            "gRPC endpoint not supported for cartezcash",
        ))
    }

    async fn get_latest_tree_state(
        &self,
        _request: tonic::Request<Empty>,
    ) -> std::result::Result<tonic::Response<TreeState>, tonic::Status> {
        tracing::info!("get_latest_tree_state called. Ignoring request");
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
        _request: tonic::Request<GetSubtreeRootsArg>,
    ) -> std::result::Result<tonic::Response<Self::GetSubtreeRootsStream>, tonic::Status> {
        tracing::info!("get_subtree_roots called. Ignoring request");
        Err(tonic::Status::unimplemented(
            "gRPC endpoint not supported for cartezcash",
        ))
    }
    async fn get_address_utxos(
        &self,
        _request: tonic::Request<GetAddressUtxosArg>,
    ) -> std::result::Result<tonic::Response<GetAddressUtxosReplyList>, tonic::Status> {
        tracing::info!("get_address_utxos called. Ignoring request");
        Err(tonic::Status::unimplemented(
            "gRPC endpoint not supported for cartezcash",
        ))
    }
    /// Server streaming response type for the GetAddressUtxosStream method.
    type GetAddressUtxosStreamStream = ReceiverStream<Result<GetAddressUtxosReply, tonic::Status>>;

    async fn get_address_utxos_stream(
        &self,
        _request: tonic::Request<GetAddressUtxosArg>,
    ) -> std::result::Result<tonic::Response<Self::GetAddressUtxosStreamStream>, tonic::Status>
    {
        tracing::info!("get_address_utxos_stream called. Ignoring request");
        Err(tonic::Status::unimplemented(
            "gRPC endpoint not supported for cartezcash",
        ))
    }

    /// Testing-only, requires lightwalletd --ping-very-insecure (do not enable in production)
    async fn ping(
        &self,
        _request: tonic::Request<Duration>,
    ) -> std::result::Result<tonic::Response<PingResponse>, tonic::Status> {
        tracing::info!("ping called. Ignoring request");
        Err(tonic::Status::unimplemented(
            "gRPC endpoint not supported for cartezcash",
        ))
    }
}
