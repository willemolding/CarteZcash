//! Response that a cartesi tower service must produce

use crate::messages::{Status, Voucher, NoticeOrReportOrException, Finish};

#[derive(Debug)]
pub struct Response {
    status: Status,
    pub notices: Vec<NoticeOrReportOrException>,
    pub reports: Vec<NoticeOrReportOrException>,
    pub vouchers: Vec<Voucher>,
}

impl Response {
    pub fn empty_accept() -> Self {
        Self {
            status: Status::Accept,
            notices: Vec::new(),
            reports: Vec::new(),
            vouchers: Vec::new(),
        }
    }

    pub fn finish_message(&self) -> Finish {
        match self.status {
            Status::Accept => Finish::accept(),
            Status::Reject => Finish::reject(),
        }
    }
}
