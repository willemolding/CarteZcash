use chrono::{DateTime, Utc};
use futures_util::future::FutureExt;
use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use tower::{Service, ServiceExt};

use zebra_chain::transaction::Transaction;
use zebra_chain::transparent;
use zebra_chain::{
    amount::{Amount, NonNegative},
    block::{Block, Header, Height},
    fmt::HexDebug,
    parameters::Network,
    work::{difficulty::CompactDifficulty, equihash::Solution},
};
use zebra_chain::{block, serialization::ZcashDeserialize};
use zebra_consensus::transaction as tx;

use crate::mt_doom;

pub type BoxError = Box<dyn std::error::Error + Send + Sync + 'static>;

pub struct TinyCashWriteService<S, V> {
    state_service: S,
    tx_verifier_service: V,
}

impl<S, V> TinyCashWriteService<S, V> {
    pub fn new(state_service: S, tx_verifier_service: V) -> Self {
        Self {
            state_service,
            tx_verifier_service,
        }
    }
}

/// The request type for the TinyCash service
pub enum Request {
    /// Add the genesis block to the state
    Genesis,
    /// Form a coinbase transaction that mints the given amount and produce a new block that includes it
    Mint {
        amount: Amount<NonNegative>,
        to: transparent::Script,
    },
    /// Produce a new block that includes the given transaction
    IncludeTransaction { transaction: Transaction },
}

/// The response type for the TinyCash service
pub struct Response {
    ///The block that was added by this state transition
    pub block: block::Block,
    /// The amount of coins that were burned by the transaction (if any) by transferring to the Mt Doom address (0x000...000)
    pub burned: Amount<NonNegative>,
}

impl<S, V> tower::Service<Request> for TinyCashWriteService<S, V>
where
    S: Service<
            zebra_state::Request,
            Response = zebra_state::Response,
            Error = zebra_state::BoxError,
        >
        + Send
        + Clone
        + 'static
        + Clone,
    S::Future: Send + 'static,
    V: Service<
            tx::Request,
            Response = tx::Response,
            Error = zebra_consensus::error::TransactionError,
        >
        + Send
        + Clone
        + 'static
        + Clone,
    V::Future: Send + 'static,
{
    type Response = Response;
    type Error = BoxError;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.state_service.poll_ready(cx)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let mut state_service = self.state_service.clone();
        let mut transaction_verifier = self.tx_verifier_service.clone();

        async move {
            let (block, height, burned) = match req {
                Request::Genesis => {
                    (
                        Block::zcash_deserialize(
                            zebra_test::vectors::BLOCK_MAINNET_GENESIS_BYTES.as_slice(),
                        )
                        .unwrap(),
                        Height(0),
                        Amount::zero(),
                    ) // here is one I prepared earlier :)
                }
                _ => {
                    let (tip_height, previous_block_hash) = match state_service
                        .ready()
                        .await?
                        .call(zebra_state::Request::Tip)
                        .await?
                    {
                        zebra_state::Response::Tip(Some(tip)) => tip,
                        _ => panic!("unexpected reponse for tip request"),
                    };

                    let height = (tip_height + 1).unwrap();


                    // Every block needs a coinbase transaction which records the height
                    // For a mint event this will also be used to mint new coins
                    // otherwise it is just an empty txn
                    // also keep track of if any coins were burned by sending to the mt doom script
                    let (transactions, burned) = match req {
                        Request::Genesis => {
                            unreachable!("genesis block is handled prior")
                        }
                        Request::Mint { amount, to } => {
                            let coinbase_tx = mint_coinbase_txn(amount, &to, height);
                            let burned = Amount::zero(); // FIX: It is conceivable that the minted coins sent straight to Mt Doom. Should handle this case
                            (vec![Arc::new(coinbase_tx)], burned)
                        }
                        Request::IncludeTransaction { transaction } => {
                            let coinbase_tx = empty_coinbase_txn(height);
                            let burned = transaction
                                .outputs()
                                .iter()
                                .filter(|output| {
                                    output.lock_script == mt_doom().create_script_from_address()
                                })
                                .map(|output| output.value)
                                .reduce(|total, elem| (total + elem).expect("overflow"))
                                .unwrap_or(Amount::zero());
                            (vec![Arc::new(coinbase_tx), Arc::new(transaction)], burned)
                        }
                    };

                    // build the block!
                    let block = Block {
                        header: Header {
                            version: 5,
                            previous_block_hash,
                            merkle_root: transactions.iter().collect(),
                            commitment_bytes: HexDebug::default(),
                            time: DateTime::<Utc>::default(),
                            difficulty_threshold: CompactDifficulty::default(),
                            nonce: HexDebug::default(),
                            solution: Solution::default(),
                        }
                        .into(),
                        transactions: transactions.clone(),
                    };
                    (block, height, burned)
                }
            };

            // the below checks are from the zebra-consensus block verifier
            // this logic mostly taken from zebra-consensus block verifier
            // https://github.com/ZcashFoundation/zebra/blob/main/zebra-consensus/src/block.rs

            let ret = block.clone();
            let block_hash = block.hash();
            let transaction_hashes: Arc<[_]> =
                block.transactions.iter().map(|t| t.hash()).collect();
            let transactions = block.transactions.clone();
            let known_utxos = Arc::new(transparent::new_ordered_outputs(
                &block,
                &transaction_hashes,
            ));

            if height > Height(0) {

                tracing::info!("Verifying transactions..... may take a while. Please wait before rescanning wallet");
                // verify the transactions
                for transaction in &transactions {
                    transaction_verifier
                        .ready()
                        .await
                        .expect("transaction verifier is always ready")
                        .call(tx::Request::Block {
                            transaction: transaction.clone(),
                            known_utxos: known_utxos.clone(),
                            height,
                            time: block.header.time,
                        })
                        .await?;
                }

                // contextually verify and commit the block
                let new_outputs = Arc::into_inner(known_utxos)
                    .expect("all verification tasks using known_utxos are complete");
                let prepared_block = zebra_state::SemanticallyVerifiedBlock {
                    block: block.into(),
                    hash: block_hash,
                    height,
                    new_outputs,
                    transaction_hashes,
                };

                tracing::info!(
                    "Appending block: height: {:?}, hash: {:?}",
                    height, block_hash
                );

                state_service
                    .ready()
                    .await?
                    .call(zebra_state::Request::CommitSemanticallyVerifiedBlock(
                        prepared_block,
                    ))
                    .await?;
            } else {
                // commit the genesis block as a checkpoint
                let prepared_block = zebra_state::CheckpointVerifiedBlock::from(Arc::new(block));
                state_service
                    .ready()
                    .await?
                    .call(zebra_state::Request::CommitCheckpointVerifiedBlock(
                        prepared_block,
                    ))
                    .await?;
            }

            // return the info about the new block
            Ok(Response { block: ret, burned })
        }
        .boxed()
    }
}

// create a new transparent v5 coinbase transaction that mints the given amount and sends it to the given address
fn mint_coinbase_txn(
    amount: Amount<NonNegative>,
    to: &transparent::Script,
    height: Height,
) -> Transaction {
    Transaction::new_v5_coinbase(
        Network::Mainnet,
        height,
        vec![(amount, to.clone())],
        Vec::new(),
    )
}

fn empty_coinbase_txn(height: Height) -> Transaction {
    mint_coinbase_txn(
        Amount::zero(),
        &mt_doom().create_script_from_address(),
        height,
    )
}
