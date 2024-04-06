use serde::{Serialize, Deserialize};
use hyper::Uri;

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")] 
pub enum Status {
    Accept,
    Reject,
}

#[derive(Debug, Serialize)]
pub struct Finish {
    pub status: Status,
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

    pub fn build_http_request(&self, host_uri: Uri) -> hyper::Request<hyper::Body> {
        let finish_uri = format!("{}/finish", host_uri);

        hyper::Request::builder()
            .method(hyper::Method::POST)
            .header(hyper::header::CONTENT_TYPE, "application/json")
            .uri(finish_uri)
            .body(hyper::Body::from(serde_json::to_string(self).unwrap()))
            .unwrap()
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case", tag = "request_type")] 
pub enum RollupRequest {
    AdvanceState {
        data: AdvanceStateData
    },
    InspectState {
        data: InspectStateData
    }
}

#[derive(Debug, Deserialize)]
pub struct AdvanceStateData {
    pub metadata: AdvanceStateMetadata,
    pub payload: String,
}

#[derive(Debug, Deserialize)]
pub struct AdvanceStateMetadata {
    pub msg_sender: ethereum_types::Address,
    pub epoch_index: usize,
    pub input_index: usize,
    pub block_number: usize,
    pub timestamp: usize,
}

#[derive(Debug, Deserialize)]
pub struct InspectStateData {
    pub payload: String
}

#[derive(Debug, Serialize)]
pub struct NoticeOrReportOrException {
    payload: String
}

#[derive(Debug, Serialize)]
pub struct Voucher {
    destination: ethereum_types::Address,
    payload: String
}
