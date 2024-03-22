use chrono::{DateTime, Utc};
use futures_util::future::FutureExt;
use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    time::Duration,
};
use tower::{timeout::Timeout, Service, ServiceExt};

use zebra_chain::transaction::Transaction;
use zebra_chain::transparent;
use zebra_chain::{
    amount::{Amount, NonNegative},
    block::{Block, Header, Height},
    fmt::HexDebug,
    parameters::Network,
    transparent::Script,
    work::{difficulty::CompactDifficulty, equihash::Solution},
};
use zebra_chain::{block, serialization::ZcashDeserialize};
use zebra_consensus::transaction as tx;

pub type BoxError = Box<dyn std::error::Error + Send + Sync + 'static>;

// outputs locked with this script are considered burned and can be released on L1
// this script pushed false to the stack so funds can never be spent
fn mt_doom() -> Script {
    Script::new(&[0])
}

const TX_VERIFY_TIMEOUT_SECS: u64 = 10;

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
        let mut transaction_verifier = Timeout::new(
            self.tx_verifier_service.clone(),
            Duration::from_secs(TX_VERIFY_TIMEOUT_SECS),
        );

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
        Network::TinyCash,
        height,
        vec![(amount, to.clone())],
        Vec::new(),
    )
}

fn empty_coinbase_txn(height: Height) -> Transaction {
    mint_coinbase_txn(
        Amount::zero(),
        &mt_doom(),
        height,
    )
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use tower::{buffer::Buffer, util::BoxService};
    use tower::ServiceExt;
    use zebra_chain::parameters::{Network, NetworkUpgrade};
    use zebra_chain::transaction::LockTime;

    // anything sent to this script can be spent by anyway. Useful for testing
    fn accepting() -> Script {
        Script::new(&[1,1])
    }

    #[tokio::test(flavor = "multi_thread")]
    #[tracing_test::traced_test]
    async fn test_genesis() {
        let network = Network::TinyCash;

        let (state_service, _, _, _) = zebra_state::init(
            zebra_state::Config::ephemeral(),
            network,
            block::Height::MAX,
            0,
        );
        let state_service = Buffer::new(state_service, 1);
        let verifier_service = tx::Verifier::new(network, state_service.clone());

        let mut tinycash =
            BoxService::new(TinyCashWriteService::new(state_service, verifier_service));

        tinycash
            .call(Request::Genesis)
            .await
            .expect("unexpected error response");
    }

    #[tokio::test(flavor = "multi_thread")]
    #[tracing_test::traced_test]
    async fn test_mint_txns_update_balance() {
        let network = Network::TinyCash;

        let (state_service, mut read_state_service, _, _) = zebra_state::init(
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

        let recipient = transparent::Address::from_pub_key_hash(Network::TinyCash, [2;20]);

        // write a bunch of blocks
        for _ in 0..100 {
            tinycash
                .ready()
                .await
                .unwrap()
                .call(Request::Mint {
                    amount: Amount::try_from(1).unwrap(),
                    to: recipient.create_script_from_address(),
                })
                .await
                .expect("unexpected error response");
        }

        let mut addresses = HashSet::new();
        addresses.insert(recipient);        // check the account balance was updated

        // check the account balance was updatedz
        let res = read_state_service.ready().await.unwrap().call(zebra_state::ReadRequest::AddressBalance(addresses.clone())).await.unwrap();
        println!("res: {:?}", res);
        assert_eq!(res, zebra_state::ReadResponse::AddressBalance(Amount::try_from(100).unwrap()));

        // check all transactions were received
        let res = read_state_service.ready().await.unwrap().call(zebra_state::ReadRequest::TransactionIdsByAddresses{ addresses, height_range: Height(0)..=Height(100) }).await.unwrap();
        println!("res: {:?}", res);
        if let zebra_state::ReadResponse::AddressesTransactionIds(transactions) = res {
            assert_eq!(transactions.len(), 100);
        } else {
            panic!("unexpected response");
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    #[tracing_test::traced_test]
    async fn test_include_transparent_transaction() {
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

        let Response { block: b1, .. } = tinycash
            .ready()
            .await
            .unwrap()
            .call(Request::Mint {
                amount: Amount::try_from(100).unwrap(),
                to: accepting(),
            })
            .await
            .expect("unexpected error response");

        println!("b1: {:?}", b1);

        let tx = build_transaction_spending(
            transparent::OutPoint {
                hash: b1.transactions[0].hash(),
                index: 0,
            },
            100.try_into().unwrap(),
        );

        tinycash
            .ready()
            .await
            .unwrap()
            .call(Request::IncludeTransaction { transaction: tx })
            .await
            .unwrap();
    }

    /// Given a `previous_outpoint` build a new transaction that should pass
    fn build_transaction_spending(
        previous_outpoint: transparent::OutPoint, // specifies how to find the UTXOs to spend
        amount: Amount<NonNegative>,
        // script_should_succeed: bool,
    ) -> Transaction {
        // A script with a single opcode that accepts the transaction (pushes true on the stack)
        let accepting_script = transparent::Script::new(&[1, 1]);
        // A script with a single opcode that rejects the transaction (OP_FALSE)
        // let rejecting_script = transparent::Script::new(&[0]);

        // Use the `previous_outpoint` as input
        let input = transparent::Input::PrevOut {
            outpoint: previous_outpoint,
            unlock_script: accepting_script.clone(),
            sequence: 0,
        };

        let output = transparent::Output {
            value: amount,
            lock_script: accepting_script,
        };

        Transaction::V5 {
            inputs: vec![input],
            outputs: vec![output],
            lock_time: LockTime::Height(Height(0)),
            expiry_height: Height(0),
            sapling_shielded_data: None,
            orchard_shielded_data: None,
            network_upgrade: NetworkUpgrade::Nu5,
        }
    }

 }