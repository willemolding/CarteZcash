use std::future::Future;
use std::pin::Pin;

pub use request::{AdvanceStateRequest, Request};
pub use response::Response;

mod request;
mod response;

pub struct CarteZcashService;

impl tower::Service<Request> for CarteZcashService {
    type Response = Response;
    type Error = anyhow::Error;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        todo!()
    }

    fn call(&mut self, req: Request) -> Self::Future {
        match req {
            Request::AdvanceState(AdvanceStateRequest::Deposit { amount, to }) => {
                println!("handling reposit request for amount {} to {}", amount, to);
                todo!()
            }
            Request::AdvanceState(AdvanceStateRequest::Transact { txn }) => {
                println!("handling transact request for txn {:?}", txn);
                todo!()
            }
            Request::InspectState => {
                println!("handling inspect state request");
                todo!()
            }
        }
    }
}
