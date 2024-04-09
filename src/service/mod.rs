use futures_util::future::FutureExt;
use std::future::Future;
use std::pin::Pin;
use tower::{BoxError, Service, ServiceExt};

pub use request::Request;

mod request;
pub struct CarteZcashService<S> {
    tiny_cash: S,
}

pub struct Response {
    pub withdrawals: Vec<(ethereum_types::Address, ethereum_types::U256)>,
}

impl<S> CarteZcashService<S> {
    pub fn new(tiny_cash: S) -> Self {
        Self { tiny_cash }
    }
}

impl From<tiny_cash::write::Response> for Response {
    fn from(res: tiny_cash::write::Response) -> Self {
        Self {
            withdrawals: res
                .burns
                .iter()
                .map(|(amount, memo)| {
                    (
                        ethereum_types::Address::from_slice(&memo.0[..20]),
                        ethereum_types::U256::from(amount.zatoshis()),
                    )
                })
                .collect(),
        }
    }
}

impl<S> Service<Request> for CarteZcashService<S>
where
    S: Service<tiny_cash::write::Request, Response = tiny_cash::write::Response, Error = BoxError>
        + Send
        + Clone
        + 'static,
    S::Future: Send + 'static,
{
    type Response = Response;
    type Error = BoxError;
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
        async move {
            match req {
                Request::Deposit { amount, to } => {
                    tracing::debug!("handling reposit request for amount {} to {}", amount, to);
                    tiny_cash
                        .ready()
                        .await?
                        .call(tiny_cash::write::Request::Mint {
                            amount,
                            to: to.create_script_from_address(),
                        })
                        .await
                        .map(|res| {
                            tracing::info!("detected burns: {:?}", res.burns);
                            res.into()
                        })
                }
                Request::Transact { txn, .. } => {
                    tracing::debug!("handling transact request for txn {:?}", txn);
                    tiny_cash
                        .ready()
                        .await?
                        .call(tiny_cash::write::Request::IncludeTransaction { transaction: txn })
                        .await
                        .map(|res| {
                            tracing::info!("detected burns: {:?}", res.burns);
                            res.into()
                        })
                }
            }
        }
        .boxed()
    }
}
