//! Response that a cartesi tower service must produce

use crate::messages::{Finish, Output, Status};

#[derive(Debug)]
pub struct Response {
    status: Status,
    pub outputs: Vec<Output>,
}

impl Response {
    pub fn empty_accept() -> Self {
        Self {
            status: Status::Accept,
            outputs: Vec::new(),
        }
    }

    pub fn empty_reject() -> Self {
        Self {
            status: Status::Reject,
            outputs: Vec::new(),
        }
    }

    pub fn add_voucher(&mut self, destination: ethereum_types::Address, payload: &[u8]) {
        self.outputs.push(Output::Voucher {
            destination,
            payload: payload.to_vec(),
        });
    }

    pub fn add_notice(&mut self, payload: &[u8]) {
        self.outputs.push(Output::Notice {
            payload: payload.to_vec(),
        });
    }

    pub fn finish_message(&self) -> Finish {
        match self.status {
            Status::Accept => Finish::accept(),
            Status::Reject => Finish::reject(),
        }
    }
}
