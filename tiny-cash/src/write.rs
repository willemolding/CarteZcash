use chrono::{DateTime, Utc};
use futures_util::future::FutureExt;
use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use tower::{Service, ServiceExt};

use zebra_chain::block;
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

        let (previous_block_hash, height, is_genesis) = match req {
            Request::Genesis => (block::Hash::default(), Height(0), true),
            _ => (block::Hash::default(), Height(1687104), false), // TODO: get the previous block hash and use that, also get the previous height
        };

        let (transactions, burned) = match req {
            Request::Genesis => {
                todo!();
            }
            Request::Mint { amount, to } => {
                let coinbase_tx = mint_coinbase_txn(amount, &to, height);
                let burned = Amount::zero();
                (vec![Arc::new(coinbase_tx)], burned)
            }
            Request::IncludeTransaction { transaction } => {
                let coinbase_tx = empty_coinbase_txn(height); // this is required for block validity
                let burned = transaction // tally up any burn spends
                    .outputs()
                    .iter()
                    .filter(|output| output.lock_script == mt_doom())
                    .map(|output| output.value)
                    .reduce(|total, elem| (total + elem).expect("overflow")) // FIX: exploit here where you could freeze the rollup by burning more than the Amount value can hold
                    .unwrap_or(Amount::zero());
                (vec![Arc::new(coinbase_tx), Arc::new(transaction)], burned)
            }
        };

        // build the block!
        let block = Block {
            header: Header {
                version: 4,
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

        println!("block: {:?}", block);

        // the below checks are from the zebra-consensus block verifier
        // this logic mostly taken from zebra-consensus block verifier
        // https://github.com/ZcashFoundation/zebra/blob/main/zebra-consensus/src/block.rs

        let block_hash = block.hash();
        let transaction_hashes: Arc<[_]> = block.transactions.iter().map(|t| t.hash()).collect();
        let known_utxos = Arc::new(transparent::new_ordered_outputs(
            &block,
            &transaction_hashes,
        ));

        async move {
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
        Network::Mainnet,
        height,
        vec![(amount, to.create_script_from_address())],
        Vec::new(),
    )
}

fn empty_coinbase_txn(height: Height) -> Transaction {
    mint_coinbase_txn(
        Amount::zero(),
        &transparent::Address::from_pub_key_hash(Network::Mainnet, [0; 20]),
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
    use zebra_chain::transaction::arbitrary::fake_v5_transactions_for_network;

    #[tokio::test(flavor = "multi_thread")]
    async fn test_mint_txn() {
        let network = Network::Mainnet;

        let (state_service, _, _, _) = zebra_state::init(
            zebra_state::Config::ephemeral(),
            network,
            block::Height::MAX,
            0,
        );
        let state_service = Buffer::new(state_service, 1);
        let verifier_service = tx::Verifier::new(network, state_service.clone());

        let tinycash = BoxService::new(TinyCashWriteService::new(state_service, verifier_service));

        tinycash
            .oneshot(Request::Mint {
                amount: Amount::try_from(100).unwrap(),
                to: transparent::Address::from_script_hash(network, [0; 20]),
            })
            .await
            .expect("unexpected error response");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_include_transaction() {
        let network = Network::Mainnet;
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

        let state_service = Buffer::new(state_service, 1);
        let verifier_service = tx::Verifier::new(network, state_service.clone());

        let tinycash = BoxService::new(TinyCashWriteService::new(state_service, verifier_service));

        let mut transaction =
            fake_v5_transactions_for_network(network, zebra_test::vectors::MAINNET_BLOCKS.iter())
                .next_back()
                .expect("At least one fake V5 transaction in the test vectors");
        if transaction
            .expiry_height()
            .expect("V5 must have expiry_height")
            < nu5_activation_height
        {
            let expiry_height = transaction.expiry_height_mut();
            *expiry_height = nu5_activation_height;
        }

        println!("transaction: {:?}", transaction);

        tinycash
            .oneshot(Request::IncludeTransaction { transaction })
            .await
            .expect("unexpected error response");
    }
}
