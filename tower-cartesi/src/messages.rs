use serde::{Serialize, Deserialize};

#[derive(Serialize)]
#[serde(rename_all = "snake_case")] 
pub enum Status {
    Accept,
    Reject,
}

#[derive(Serialize)]
pub struct Finish {
    status: Status,
}

impl Finish {
    pub fn accept() -> Self {
        Self {
            status: Status::Accept,
        }
    }

    pub fn reject() -> Self {
        Self {
            status: Status::Reject,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")] 
pub enum RollupRequest {
    AdvanceState {
        data: AdvanceStateData
    },
    InspectState {
        data: InspectStateData
    }
}

#[derive(Deserialize)]
pub struct AdvanceStateData {
    pub metadata: AdvanceStateMetadata,
    pub payload: String,
}

#[derive(Deserialize)]
pub struct AdvanceStateMetadata {
    pub msg_sender: ethereum_types::Address,
    pub epoch_index: usize,
    pub input_index: usize,
    pub block_number: usize,
    pub timestamp: usize,
}

#[derive(Deserialize)]
pub struct InspectStateData {
    pub payload: String
}

#[derive(Serialize)]
pub struct NoticeOrReportOrException {
    payload: String
}

#[derive(Serialize)]
pub struct Voucher {
    destination: ethereum_types::Address,
    payload: String
}
