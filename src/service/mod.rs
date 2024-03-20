use std::future::Future;
use futures_util::future::FutureExt;
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
    type Error = tiny_cash::write::BoxError;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        todo!()
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let mut tiny_cash = self.tiny_cash.clone();
        async move {
            let res = match req {
                Request::AdvanceState(AdvanceStateRequest::Deposit { amount, to }) => {
                    println!("handling reposit request for amount {} to {}", amount, to);
                    tiny_cash.call(tiny_cash::write::Request::Mint { amount, to })
                }
                Request::AdvanceState(AdvanceStateRequest::Transact { txn }) => {
                    println!("handling transact request for txn {:?}", txn);
                    tiny_cash.call(tiny_cash::write::Request::IncludeTransaction { transaction: txn })
                }
                Request::InspectState => {
                    println!("handling inspect state request");
                    todo!()
                }
            }.await;
            res.map(|res| Response::Accept { burned: res.burned.into() })
        }.boxed()
    }
}
