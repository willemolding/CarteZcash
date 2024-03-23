
pub type Payload = Vec<u8>;

pub struct AdvanceStateMetadata {
    pub msg_sender: ethereum_types::Address,
    pub input_index: u64,
    pub block_numbner: u64,
    pub timestamp: u64,
}

pub enum Request {
    AdvanceState {
        payload: Payload,
        metadata: AdvanceStateMetadata,
    },
    InspectState {
        payload: Payload,
    },
}

#[derive(Default)]
pub struct AcceptResponse {
    notices: Vec<Payload>,
    reports: Vec<Payload>,
    vouchers: Vec<Payload>,
}

pub type Response = Result<AcceptResponse, anyhow::Error>;

async fn listen_for_requests<S>(server_addr: hyper::Uri)
    where 
    S: tower::Service<Request, Response = Response, Error = anyhow::Error> + Send + 'static
{
    let client = hyper::Client::new();
    // let mut status = Ok(AcceptResponse::default());
    loop {
        // let response = client.request(status.host_request(&server_addr)).await?;
    }
}
