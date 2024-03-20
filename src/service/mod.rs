use std::future::Future;
use std::pin::Pin;
use tower::Service;

pub use request::{AdvanceStateRequest, Request};
pub use response::Response;

mod request;
mod response;

pub struct CarteZcashService<S> {
    tiny_cash: S,
}

impl<S> CarteZcashService<S> {
    pub fn new(tiny_cash: S) -> Self {
        Self { tiny_cash }
    }
}

impl<S> Service<Request> for CarteZcashService<S>
where
    S: Service<
            tiny_cash::write::Request,
            Response = tiny_cash::write::Response,
            Error = tiny_cash::write::BoxError,
        >
        + Send
        + Clone
        + 'static,
    S::Future: Send + 'static,
{
    type Response = Response;
    type Error = anyhow::Error;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
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
