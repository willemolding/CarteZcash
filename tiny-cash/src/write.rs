use chrono::{DateTime, Utc};
use futures_util::future::FutureExt;
use std::{
    collections::HashMap,
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use tower::{Service, ServiceExt};

use zebra_chain::transparent;
use zebra_chain::{
    amount::{Amount, NonNegative},
    block::{Block, Header, Height},
    fmt::HexDebug,
    parameters::{Network, NetworkUpgrade},
    transaction::HashType,
    transparent::{OrderedUtxo, OutPoint},
    work::{difficulty::CompactDifficulty, equihash::Solution},
};
use zebra_chain::{block, serialization::ZcashDeserialize};
use zebra_chain::{
    transaction::{Memo, Transaction},
    transparent::Script,
};
use zebra_consensus::script;
use zebra_consensus::transaction as tx;
use zebra_consensus::transaction::Verifier as TxVerifier;

use crate::extract_burn_info;

pub type BoxError = Box<dyn std::error::Error + Send + Sync + 'static>;

pub struct TinyCashWriteService<S> {
    state_service: S,

    tip_height: Option<Height>,
    tip_hash: Option<block::Hash>,

    // Set of all unspent transparent outputs.
    // This is expected to grow but very slowly since transparent outputs are only created
    // by deposits and destroyed by subsequent spends.
    utxos_set: HashMap<OutPoint, OrderedUtxo>,
}

impl<S> TinyCashWriteService<S> {
    pub fn new(state_service: S) -> Self {
        Self {
            state_service,

            tip_height: None,
            tip_hash: None,
            utxos_set: HashMap::new(),
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
    pub burns: Vec<(Amount<NonNegative>, Memo)>,
}

impl<S> tower::Service<Request> for TinyCashWriteService<S>
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

        let tip_height = self.tip_height;
        let previous_block_hash = self.tip_hash.unwrap_or(Default::default());

        let height = tip_height.map(|h| h.next().unwrap()).unwrap_or(Height(0));

        let (block, burns, tx_to_verify) = match req {
            Request::Genesis => (genesis_block(), Vec::new(), None),
            Request::Mint { amount, to } => {
                let block = build_mint_block(height, previous_block_hash, amount, to);
                let burns = Vec::new();
                (block, burns, None)
            }
            Request::IncludeTransaction { transaction } => {
                let burns = transaction
                    .orchard_actions()
                    .filter_map(extract_burn_info)
                    .collect();
                let block = build_transact_block(height, previous_block_hash, transaction.clone());
                (block, burns, Some(transaction))
            }
        };

        // the below checks are from the zebra-consensus block verifier
        // this logic mostly taken from zebra-consensus block verifier
        // https://github.com/ZcashFoundation/zebra/blob/main/zebra-consensus/src/block.rs

        let ret = block.clone();
        let block_hash = block.hash();

        self.tip_hash = Some(block_hash);
        self.tip_height = Some(height);
        let outputs_spent = if let Some(tx) = tx_to_verify.as_ref() {
            self.outputs_spent_by_transaction(tx.clone().into())
        } else {
            Vec::new()
        };

        // build the set of new UTXOs this block creates and add to the global state
        let transaction_hashes: Arc<[_]> = block.transactions.iter().map(|t| t.hash()).collect();
        let known_utxos = Arc::new(transparent::new_ordered_outputs(
            &block,
            &transaction_hashes,
        ));
        let new_outputs = Arc::into_inner(known_utxos)
            .expect("all verification tasks using known_utxos are complete");
        tracing::info!("Adding new UTXOs to the set: {:?}", new_outputs);
        self.utxos_set
            .extend(new_outputs.iter().map(|(k, v)| (k.clone(), v.clone())));

        async move {
            if height > Height(0) {
                // verify the transaction if required
                if let Some(tx) = tx_to_verify {
                    tracing::info!("Verifying transaction");
                    Self::verify_transaction(tx.into(), height, outputs_spent.as_slice()).await?;
                    tracing::info!("Transaction passed!");
                }

                // contextually verify and commit the block
                let prepared_block = zebra_state::SemanticallyVerifiedBlock {
                    block: block.into(),
                    hash: block_hash,
                    height,
                    new_outputs,
                    transaction_hashes,
                };

                tracing::info!(
                    "Appending block: height: {:?}, hash: {:?}",
                    height,
                    block_hash
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
            Ok(Response { block: ret, burns })
        }
        .boxed()
    }
}

impl<S> TinyCashWriteService<S> {
    pub fn outputs_spent_by_transaction(&self, tx: Arc<Transaction>) -> Vec<transparent::Output> {
        tx.inputs()
            .iter()
            .filter_map(|input| {
                if let Some(outpoint) = input.outpoint() {
                    self.utxos_set
                        .get(&outpoint)
                        .map(|utxo| utxo.as_ref().output.clone())
                } else {
                    None
                }
            })
            .collect()
    }
}

impl<S> TinyCashWriteService<S>
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
{
    async fn verify_transaction(
        tx: Arc<Transaction>,
        height: Height,
        all_previous_outputs: &[transparent::Output],
    ) -> Result<(), BoxError> {
        let async_checks = match tx.as_ref() {
            Transaction::V1 { .. }
            | Transaction::V2 { .. }
            | Transaction::V3 { .. }
            | Transaction::V4 { .. } => {
                panic!("Unsupported transaction version");
            }
            Transaction::V5 {
                sapling_shielded_data,
                orchard_shielded_data,
                ..
            } => {
                if sapling_shielded_data.is_some() {
                    panic!("Sapling shielded data is not supported");
                }
                let shielded_sighash = tx.sighash(
                    NetworkUpgrade::Nu5,
                    HashType::ALL,
                    all_previous_outputs,
                    None,
                );
                TxVerifier::<S>::verify_transparent_inputs_and_outputs(
                    &tx::Request::Block {
                        transaction: tx.clone(),
                        known_utxos: HashMap::new().into(),
                        height,
                        time: DateTime::<Utc>::default(),
                    },
                    Network::Mainnet,
                    script::Verifier, // TODO: Maybe try and reuse this
                    zebra_script::CachedFfiTransaction::new(
                        tx.clone(),
                        all_previous_outputs.to_vec(),
                    )
                    .into(),
                )?
                .and(TxVerifier::<S>::verify_orchard_shielded_data(
                    &orchard_shielded_data,
                    &shielded_sighash,
                )?)
            }
        };
        async_checks.check().await
    }
}

fn build_mint_block(
    height: Height,
    previous_block_hash: block::Hash,
    amount: Amount<NonNegative>,
    to: transparent::Script,
) -> Block {
    let coinbase_tx = mint_coinbase_txn(amount, &to, height);
    build_block(previous_block_hash, vec![Arc::new(coinbase_tx)])
}

fn build_transact_block(
    height: Height,
    previous_block_hash: block::Hash,
    transaction: Transaction,
) -> Block {
    let coinbase_tx = empty_coinbase_txn(height);
    build_block(
        previous_block_hash,
        vec![Arc::new(coinbase_tx), Arc::new(transaction)],
    )
}

fn genesis_block() -> Block {
    Block::zcash_deserialize(zebra_test::vectors::BLOCK_MAINNET_GENESIS_BYTES.as_slice()).unwrap()
}

fn build_block(previous_block_hash: block::Hash, transactions: Vec<Arc<Transaction>>) -> Block {
    Block {
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
        transactions,
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
    mint_coinbase_txn(Amount::zero(), &Script::new(&[0x0; 32]), height)
}
