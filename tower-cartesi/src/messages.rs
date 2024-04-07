use hyper::Uri;
use serde::{Deserialize, Serialize};

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
    AdvanceState { data: AdvanceStateData },
    InspectState { data: InspectStateData },
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
    pub payload: String,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum Output {
    Notice {
        #[serde(serialize_with = "hexify")]
        payload: Vec<u8>,
    },
    Report {
        #[serde(serialize_with = "hexify")]
        payload: Vec<u8>,
    },
    Voucher {
        destination: ethereum_types::Address,
        #[serde(serialize_with = "hexify")]
        payload: Vec<u8>,
    },
}

fn hexify<S>(data: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&format!("0x{}", hex::encode(data)))
}

impl Output {
    pub fn build_http_request(&self, host_uri: Uri) -> hyper::Request<hyper::Body> {
        let uri = match self {
            Self::Notice { .. } => format!("{}/notice", host_uri),
            Self::Report { .. } => format!("{}/report", host_uri),
            Self::Voucher { .. } => format!("{}/voucher", host_uri),
        };

        hyper::Request::builder()
            .method(hyper::Method::POST)
            .header(hyper::header::CONTENT_TYPE, "application/json")
            .uri(uri)
            .body(hyper::Body::from(serde_json::to_string(self).unwrap()))
            .unwrap()
    }
}
