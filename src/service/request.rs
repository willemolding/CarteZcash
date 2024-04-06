use ethereum_types::U256;

use hex::ToHex;
use zebra_chain::amount::{Amount, NonNegative};
use zebra_chain::serialization::ZcashDeserialize;
use zebra_chain::transaction::Transaction;
use zebra_chain::transparent::Address;

/// Requests that can be received from the L1
/// will be either EtherTransfer {"request_type":"advance_state","data":{"metadata":{"msg_sender":"0xffdbe43d4c855bf7e0f105c400a50857f53ab044","epoch_index":0,"input_index":0,"block_number":11,"timestamp":1710913093},"payload":"0xf39fd6e51aad88f6f4ce6ab8827279cfffb92266000000000000000000000000000000000000000000000001314fb37062980000"}}
///      or generic data message {"request_type":"advance_state","data":{"metadata":{"msg_sender":"0xf39fd6e51aad88f6f4ce6ab8827279cfffb92266","epoch_index":0,"input_index":1,"block_number":122,"timestamp":1710913648},"payload":"0xffff"}}
#[derive(Clone)]
pub enum Request {
    Deposit {
        amount: Amount<NonNegative>,
        to: Address,
    },
    Transact {
        withdraw_address: ethereum_types::Address,
        txn: Transaction,
    },
}

impl std::fmt::Debug for Request {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Request::Deposit { amount, to } => {
                write!(f, "Deposit {} to {}", amount, to)
            }
            Request::Transact {
                withdraw_address,
                txn,
            } => {
                write!(
                    f,
                    "Transact hash {} with withdrawal address {}",
                    txn.hash(),
                    withdraw_address
                )
            }
        }
    }
}

const ETH_DEPOSIT_ADDR: &str = "ffdbe43d4c855bf7e0f105c400a50857f53ab044";
const INBOX_CONTRACT_ADDR: &str = "f39fd6e51aad88f6f4ce6ab8827279cfffb92266";

impl TryFrom<(tower_cartesi::AdvanceStateMetadata, Vec<u8>)> for Request {
    type Error = anyhow::Error;

    fn try_from(
        (metadata, payload): (tower_cartesi::AdvanceStateMetadata, Vec<u8>),
    ) -> Result<Self, Self::Error> {
        match hex::encode(metadata.msg_sender.as_bytes()).as_str() {
            ETH_DEPOSIT_ADDR => {
                /*  encoding as determined by the Cartesi Eth deposit contract
                abi.encodePacked(
                    sender, //              20B
                    value, //               32B
                    execLayerData //        arbitrary size
                );
                */

                let _sender = &payload[0..20];
                let value = U256::from_big_endian(&payload[20..52])
                    .checked_div(U256::from(10_000_000_000_u64))
                    .unwrap(); // 1 ZEC is 100_000_000 units while 1 ETH is 10^18. So we divide by 10^10 so that 1 ETH is 1 ZEC

                let dest_t_address = Address::from_pub_key_hash(
                    zebra_chain::parameters::Network::Mainnet,
                    payload[52..].try_into().unwrap(),
                );
                let amount = Amount::try_from(value.as_u64())?; // FIX: This is going to panic if too much eth is sent

                tracing::info!(
                    "Received deposit request for {} to {}",
                    value,
                    dest_t_address
                );

                Ok(Request::Deposit {
                    amount,
                    to: dest_t_address,
                })
            }
            INBOX_CONTRACT_ADDR => {
                /* Encoding
                abi.encodePacked(
                    withdraw address, // 20B
                    transaction bytes // arbitrary size
                 */

                let withdraw_address =
                    ethereum_types::Address::from_slice(payload[0..20].try_into().unwrap());

                let txn = zebra_chain::transaction::Transaction::zcash_deserialize(&payload[20..])?;

                tracing::info!(
                    "Received transaction request {} send burns to {}",
                    txn.hash(),
                    withdraw_address
                );

                Ok(Request::Transact {
                    withdraw_address,
                    txn,
                })
            }
            _ => anyhow::bail!("unrecognised sender {}", metadata.msg_sender.to_string()),
        }
    }
}
