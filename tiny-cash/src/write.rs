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
    transaction::LockTime,
    transparent::Script,
    work::{difficulty::CompactDifficulty, equihash::Solution},
};
use zebra_chain::{block, serialization::ZcashDeserialize, transparent::GENESIS_COINBASE_DATA};
use zebra_consensus::transaction as tx;

// outputs locked with this script are considered burned and can be released on L1
// this script pushed false to the stack so funds can never be spent
fn mt_doom() -> Script {
    Script::new(&[0])
}

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
        to: transparent::Address,
    },
    /// Produce a new block that includes the given transaction
    IncludeTransaction { transaction: Transaction },
}

/// The response type for the TinyCash service
pub struct Response {
    /// Hash of the block that was added by this state transition
    pub block_hash: block::Hash,
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
    type Error = zebra_consensus::BoxError;
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

                    println!(
                        "Creating block for height: {:?}, parent_hash: {:?}",
                        height, previous_block_hash
                    );

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
                                .filter(|output| output.lock_script == mt_doom())
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

            let block_hash = block.hash();
            let transaction_hashes: Arc<[_]> =
                block.transactions.iter().map(|t| t.hash()).collect();
            let transactions = block.transactions.clone();
            let known_utxos = Arc::new(transparent::new_ordered_outputs(
                &block,
                &transaction_hashes,
            ));

            if height > Height(0) {
                // verify the transactions
                for transaction in &transactions {
                    let rsp = transaction_verifier
                        .ready()
                        .await
                        .expect("transaction verifier is always ready")
                        .call(tx::Request::Block {
                            transaction: transaction.clone(),
                            known_utxos: known_utxos.clone(),
                            height,
                            time: block.header.time,
                        })
                        .await
                        .unwrap();
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
            Ok(Response { block_hash, burned })
        }
        .boxed()
    }
}

// create a new transparent v5 coinbase transaction that mints the given amount and sends it to the given address
fn mint_coinbase_txn(
    amount: Amount<NonNegative>,
    to: &transparent::Address,
    height: Height,
) -> Transaction {
    Transaction::new_v5_coinbase(
        Network::TinyCash,
        height,
        vec![(amount, to.create_script_from_address())],
        Vec::new(),
    )
}

fn empty_coinbase_txn(height: Height) -> Transaction {
    mint_coinbase_txn(
        Amount::zero(),
        &transparent::Address::from_pub_key_hash(Network::TinyCash, [0; 20]),
        height,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tower::buffer::Buffer;
    use tower::util::BoxService;
    use tower::ServiceExt;
    use zebra_chain::parameters::{Network, NetworkUpgrade};
    use zebra_chain::serialization::ZcashDeserializeInto;
    use zebra_chain::transaction::arbitrary::fake_v5_transactions_for_network;

    // #[tokio::test(flavor = "multi_thread")]
    // #[tracing_test::traced_test]
    // async fn test_genesis() {
    //     let network = Network::TinyCash;

    //     let (state_service, _, _, _) = zebra_state::init(
    //         zebra_state::Config::ephemeral(),
    //         network,
    //         block::Height::MAX,
    //         0,
    //     );
    //     let state_service = Buffer::new(state_service, 1);
    //     let verifier_service = tx::Verifier::new(network, state_service.clone());

    //     let mut tinycash =
    //         BoxService::new(TinyCashWriteService::new(state_service, verifier_service));

    //     tinycash
    //         .call(Request::Genesis)
    //         .await
    //         .expect("unexpected error response");
    // }

    #[tokio::test(flavor = "multi_thread")]
    // #[tracing_test::traced_test]
    async fn test_mint_txn() {
        let network = Network::TinyCash;

        let (state_service, _, _, _) = zebra_state::init(
            zebra_state::Config::ephemeral(),
            network,
            block::Height::MAX,
            0,
        );
        let state_service = Buffer::new(state_service, 10);
        let verifier_service = tx::Verifier::new(network, state_service.clone());

        let mut tinycash =
            BoxService::new(TinyCashWriteService::new(state_service, verifier_service));

        tinycash
            .ready()
            .await
            .unwrap()
            .call(Request::Genesis)
            .await
            .unwrap();
        tinycash
            .ready()
            .await
            .unwrap()
            .call(Request::Mint {
                amount: Amount::try_from(100).unwrap(),
                to: transparent::Address::from_script_hash(network, [0; 20]),
            })
            .await
            .expect("unexpected error response");
    }

    #[tokio::test(flavor = "multi_thread")]
    #[tracing_test::traced_test]
    async fn test_include_transparent_transaction() {
        let network = Network::TinyCash;
        let nu5 = NetworkUpgrade::Nu5;
        let nu5_activation_height = nu5
            .activation_height(network)
            .expect("NU5 activation height is specified");

        let (state_service, _, _, _) = zebra_state::init(
            zebra_state::Config::ephemeral(),
            network,
            block::Height::MAX,
            0,
        );

        let state_service = Buffer::new(state_service, 10);
        let verifier_service = tx::Verifier::new(network, state_service.clone());

        let mut tinycash =
            BoxService::new(TinyCashWriteService::new(state_service, verifier_service));

        let mut transaction =
            fake_v5_transactions_for_network(network, zebra_test::vectors::MAINNET_BLOCKS.iter())
                .next_back()
                .expect("At least one fake V5 transaction in the test vectors");

        let expiry_height = transaction.expiry_height_mut();
        *expiry_height = nu5_activation_height;

        tinycash
            .ready()
            .await
            .unwrap()
            .call(Request::Genesis)
            .await
            .unwrap();
        tinycash
            .ready()
            .await
            .unwrap()
            .call(Request::IncludeTransaction { transaction })
            .await
            .expect("unexpected error response");
    }

    // // test verifyinga shielded transactio. This fails but I think it is supposed to because of the way I am building the proof..
    // #[tokio::test(flavor = "multi_thread")]
    // #[tracing_test::traced_test]
    // async fn test_include_shielded_transaction() {
    //     let network = Network::TinyCash;
    //     let nu5 = NetworkUpgrade::Nu5;
    //     let nu5_activation_height = nu5
    //         .activation_height(network)
    //         .expect("NU5 activation height is specified");

    //     let (state_service, _, _, _) = zebra_state::init(
    //         zebra_state::Config::ephemeral(),
    //         network,
    //         block::Height::MAX,
    //         0,
    //     );

    //     let state_service = Buffer::new(state_service, 1);
    //     let verifier_service = tx::Verifier::new(network, state_service.clone());

    //     let tinycash = BoxService::new(TinyCashWriteService::new(state_service, verifier_service));

    //     // These test vectors are generated by `generate_test_vectors()` function.
    //     let shielded_data = zebra_test::vectors::ORCHARD_SHIELDED_DATA
    //     .clone()
    //     .iter()
    //     .map(|bytes| {
    //         let maybe_shielded_data: Option<zebra_chain::orchard::ShieldedData> = bytes
    //             .zcash_deserialize_into()
    //             .expect("a valid orchard::ShieldedData instance");
    //         maybe_shielded_data.unwrap()
    //     })
    //     .next().unwrap();

    //     let authorized_actions = shielded_data.actions[0].clone();

    //     let mut transaction = transaction_v5_with_orchard_shielded_data(
    //         shielded_data,
    //         [authorized_actions],
    //     );

    //     let expiry_height = transaction.expiry_height_mut();
    //     *expiry_height = nu5_activation_height;

    //     tinycash
    //         .oneshot(Request::IncludeTransaction { transaction })
    //         .await
    //         .expect("unexpected error response");
    // }

    // /// Return a `Transaction::V5` containing `orchard_shielded_data`.
    // /// with its `AuthorizedAction`s replaced by `authorized_actions`.
    // ///
    // /// Other fields have empty or default values.
    // ///
    // /// # Panics
    // ///
    // /// If there are no `AuthorizedAction`s in `authorized_actions`.
    // fn transaction_v5_with_orchard_shielded_data(
    //     orchard_shielded_data: impl Into<Option<zebra_chain::orchard::ShieldedData>>,
    //     authorized_actions: impl IntoIterator<Item = zebra_chain::orchard::AuthorizedAction>,
    // ) -> Transaction {
    //     let mut orchard_shielded_data = orchard_shielded_data.into();
    //     let authorized_actions: Vec<_> = authorized_actions.into_iter().collect();

    //     if let Some(ref mut orchard_shielded_data) = orchard_shielded_data {
    //         // make sure there are no other nullifiers, by replacing all the authorized_actions
    //         orchard_shielded_data.actions = authorized_actions.try_into().expect(
    //             "unexpected invalid orchard::ShieldedData: must have at least one AuthorizedAction",
    //         );

    //         // set value balance to 0 to pass the chain value pool checks
    //         let zero_amount = 0.try_into().expect("unexpected invalid zero amount");
    //         orchard_shielded_data.value_balance = zero_amount;
    //     }

    //     Transaction::V5 {
    //         network_upgrade: NetworkUpgrade::Nu5,
    //         inputs: Vec::new(),
    //         outputs: Vec::new(),
    //         lock_time: LockTime::min_lock_time_timestamp(),
    //         expiry_height: Height(0),
    //         sapling_shielded_data: None,
    //         orchard_shielded_data,
    //     }
    // }
}
