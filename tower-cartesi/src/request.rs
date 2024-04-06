//! Request a cartesi tower service must handle

use crate::messages::{AdvanceStateMetadata, RollupRequest};

#[derive(Debug)]
pub enum Request {
    AdvanceState {
        metadata: AdvanceStateMetadata,
        payload: Vec<u8>,
    },
    InspectState {
        payload: Vec<u8>,
    },
}

impl TryFrom<RollupRequest> for Request {
    type Error = Box<dyn std::error::Error>;

    fn try_from(request: RollupRequest) -> Result<Self, Self::Error> {
        match request {
            RollupRequest::AdvanceState { data } => Ok(Request::AdvanceState {
                metadata: data.metadata,
                payload: hex::decode(data.payload.trim_start_matches("0x"))?,
            }),
            RollupRequest::InspectState { data } => Ok(Request::InspectState {
                payload: hex::decode(data.payload.trim_start_matches("0x"))?,
            }),
        }
    }
}
