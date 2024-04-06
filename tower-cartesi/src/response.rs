//! Response that a cartesi tower service must produce

use crate::messages::{Status, Voucher, NoticeOrReportOrException};

pub struct Response {
    status: Status,
    notices: Vec<NoticeOrReportOrException>,
    reports: Vec<NoticeOrReportOrException>,
    vouchers: Vec<Voucher>,
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
}
