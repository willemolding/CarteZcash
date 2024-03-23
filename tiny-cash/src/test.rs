use std::collections::HashSet;

use crate::write::*;
use tower::ServiceExt;
use tower::{buffer::Buffer, util::BoxService};
use zebra_chain::parameters::{Network, NetworkUpgrade};
use zebra_chain::transaction::LockTime;
use zebra_chain::transparent::Script;

use tower::Service;

use zebra_chain::transaction::Transaction;
use zebra_chain::transparent;
use zebra_chain::{
    amount::{Amount, NonNegative},
    block,
    block::Height,
};
use zebra_consensus::transaction as tx;

// anything sent to this script can be spent by anyway. Useful for testing
fn accepting() -> Script {
    Script::new(&[1, 1])
}

#[tokio::test(flavor = "multi_thread")]
#[tracing_test::traced_test]
async fn test_genesis() {
    let network = Network::Mainnet;

    let (state_service, _, _, _) = zebra_state::init(
        zebra_state::Config::ephemeral(),
        network,
        block::Height::MAX,
        0,
    );
    let state_service = Buffer::new(state_service, 1);
    let verifier_service = tx::Verifier::new(network, state_service.clone());

    let mut tinycash = BoxService::new(TinyCashWriteService::new(state_service, verifier_service));

    tinycash
        .call(Request::Genesis)
        .await
        .expect("unexpected error response");
}

#[tokio::test(flavor = "multi_thread")]
#[tracing_test::traced_test]
async fn test_mint_txns_update_balance() {
    let network = Network::Mainnet;

    let (state_service, mut read_state_service, _, _) = zebra_state::init(
        zebra_state::Config::ephemeral(),
        network,
        block::Height::MAX,
        0,
    );
    let state_service = Buffer::new(state_service, 10);
    let verifier_service = tx::Verifier::new(network, state_service.clone());

    let mut tinycash = BoxService::new(TinyCashWriteService::new(state_service, verifier_service));

    tinycash
        .ready()
        .await
        .unwrap()
        .call(Request::Genesis)
        .await
        .unwrap();

    let recipient = transparent::Address::from_pub_key_hash(Network::Mainnet, [2; 20]);

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
    addresses.insert(recipient); // check the account balance was updated

    // check the account balance was updatedz
    let res = read_state_service
        .ready()
        .await
        .unwrap()
        .call(zebra_state::ReadRequest::AddressBalance(addresses.clone()))
        .await
        .unwrap();
    println!("res: {:?}", res);
    assert_eq!(
        res,
        zebra_state::ReadResponse::AddressBalance(Amount::try_from(100).unwrap())
    );

    // check all transactions were received
    let res = read_state_service
        .ready()
        .await
        .unwrap()
        .call(zebra_state::ReadRequest::TransactionIdsByAddresses {
            addresses,
            height_range: Height(0)..=Height(100),
        })
        .await
        .unwrap();
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
    let network = Network::Mainnet;

    let (state_service, _, _, _) = zebra_state::init(
        zebra_state::Config::ephemeral(),
        network,
        block::Height::MAX,
        0,
    );

    let state_service = Buffer::new(state_service, 10);
    let verifier_service = tx::Verifier::new(network, state_service.clone());

    let mut tinycash = BoxService::new(TinyCashWriteService::new(state_service, verifier_service));

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
