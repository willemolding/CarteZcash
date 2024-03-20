use json::JsonValue;

use zebra_chain::amount::Amount;
use zebra_chain::serialization::ZcashDeserialize;
use zebra_chain::transaction::Transaction;
use zebra_chain::transparent::Address;

#[derive(Debug)]
pub enum Request {
    AdvanceState(AdvanceStateRequest),
    InspectState,
}

impl TryFrom<JsonValue> for Request {
    type Error = anyhow::Error;

    fn try_from(req: JsonValue) -> Result<Self, Self::Error> {
        match req["request_type"].as_str() {
            Some("advance_state") => {
                let advance_state = AdvanceStateRequest::try_from(req)?;
                Ok(Request::AdvanceState(advance_state))
            }
            Some("inspect_state") => Ok(Request::InspectState),
            _ => anyhow::bail!("Invalid request type"),
        }
    }
}

/// Requests that can be received from the L1
/// will be either EtherTransfer {"request_type":"advance_state","data":{"metadata":{"msg_sender":"0xffdbe43d4c855bf7e0f105c400a50857f53ab044","epoch_index":0,"input_index":0,"block_number":11,"timestamp":1710913093},"payload":"0xf39fd6e51aad88f6f4ce6ab8827279cfffb92266000000000000000000000000000000000000000000000001314fb37062980000"}}
///      or generic data message {"request_type":"advance_state","data":{"metadata":{"msg_sender":"0xf39fd6e51aad88f6f4ce6ab8827279cfffb92266","epoch_index":0,"input_index":1,"block_number":122,"timestamp":1710913648},"payload":"0xffff"}}
#[derive(Debug)]
pub enum AdvanceStateRequest {
    Deposit { amount: Amount, to: Address },
    Transact { txn: Transaction },
}

const ETH_DEPOSIT_ADDR: &str = "0xffdbe43d4c855bf7e0f105c400a50857f53ab044";
const INBOX_CONTRACT_ADDR: &str = "0xf39fd6e51aad88f6f4ce6ab8827279cfffb92266";

impl TryFrom<JsonValue> for AdvanceStateRequest {
    type Error = anyhow::Error;

    fn try_from(req: JsonValue) -> Result<Self, Self::Error> {
        if req["request_type"] != "advance_state" {
            anyhow::bail!("Invalid request type");
        }
        match req["data"]["metadata"]["msg_sender"].as_str() {
            Some(ETH_DEPOSIT_ADDR) => {
                todo!();
            }
            Some(INBOX_CONTRACT_ADDR) => {
                let hex = req["data"]["payload"]
                    .as_str().unwrap();

                Ok(AdvanceStateRequest::Transact {
                    txn: zcash_txn_from_hex(hex)?,
                })
            }
            _ => anyhow::bail!("unrecognised sender"),
        }
    }
}

pub fn zcash_txn_from_hex(hex: &str) -> Result<Transaction, anyhow::Error> {
    let bytes = hex::decode(hex.trim_start_matches("0x"))?;
    let txn = zebra_chain::transaction::Transaction::zcash_deserialize(&mut bytes.as_slice())?;
    Ok(txn)
}
