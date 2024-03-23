use futures_util::future::FutureExt;
use std::future::Future;
use std::pin::Pin;
use tower::{Service, ServiceExt};

pub use request::{AdvanceStateRequest, Request};
pub use response::Response;

mod request;
mod response;

pub struct CarteZcashService<S, SR> {
    tiny_cash: S,
    state_read_service: SR,
}

impl<S, SR> CarteZcashService<S, SR> {
    pub fn new(tiny_cash: S, state_read_service: SR) -> Self {
        Self {
            tiny_cash,
            state_read_service,
        }
    }
}

impl<S, SR> Service<Request> for CarteZcashService<S, SR>
where
    S: Service<
            tiny_cash::write::Request,
            Response = tiny_cash::write::Response,
            Error = tiny_cash::write::BoxError,
        > + Send
        + Clone
        + 'static,
    S::Future: Send + 'static,
    SR: Service<
            zebra_state::ReadRequest,
            Response = zebra_state::ReadResponse,
            Error = zebra_state::BoxError,
        > + Send
        + Clone
        + 'static,
    SR::Future: Send + 'static,
{
    type Response = Response;
    type Error = tiny_cash::write::BoxError;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.tiny_cash.poll_ready(cx)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let mut tiny_cash = self.tiny_cash.clone();
        let mut state_read_service = self.state_read_service.clone();
        async move {
            match req {
                Request::AdvanceState(AdvanceStateRequest::Deposit { amount, to }) => {
                    tracing::info!("handling reposit request for amount {} to {}", amount, to);
                    tiny_cash
                        .ready()
                        .await?
                        .call(tiny_cash::write::Request::Mint {
                            amount,
                            to: to.create_script_from_address(),
                        })
                        .await
                        .map(|res| Response::Accept {
                            burned: res.burned.into(),
                        })
                }
                Request::AdvanceState(AdvanceStateRequest::Transact { txn, .. }) => {
                    tracing::info!("handling transact request for txn {:?}", txn);
                    tiny_cash
                        .ready()
                        .await?
                        .call(tiny_cash::write::Request::IncludeTransaction { transaction: txn })
                        .await
                        .map(|res| Response::Accept {
                            burned: res.burned.into(),
                        })
                }
                Request::InspectState(request) => {
                    tracing::info!("handling inspect state request");
                    state_read_service
                        .ready()
                        .await?
                        .call(request)
                        .await
                        .map(|res| {
                            let mut payload = Vec::new();
                            ciborium::into_writer(&res, &mut payload).unwrap();
                            Response::Report { payload }
                        })
                }
            }
        }
        .boxed()
    }
}
