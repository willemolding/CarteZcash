//! Request a cartesi tower service must handle

use crate::messages::AdvanceStateMetadata;

pub enum Request {
    AdvanceState {
        metadata: AdvanceStateMetadata,
        payload: Vec<u8>
    },
    InspectState {
        payload: Vec<u8>
    }
}