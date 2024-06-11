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
    pub block: tiny_cash::SemanticallyVerifiedBlock,
}

impl<S> CarteZcashService<S> {
    pub fn new(tiny_cash: S) -> Self {
        Self { tiny_cash }
    }
}

impl From<tiny_cash::service::Response> for Response {
    fn from(res: tiny_cash::service::Response) -> Self {
        Self {
            withdrawals: res
                .burns
                .iter()
                .filter_map(|(amount, memo)| {
                    if let Ok(address_bytes) = hex::decode(&memo.0[0..40]) {
                        // expect unicode hex no 0x prefix (inefficient). skip the version byte at the start
                        Some((
                            ethereum_types::Address::from_slice(&address_bytes),
                            ethereum_types::U256::from(amount.zatoshis()),
                        ))
                    } else {
                        tracing::debug!(
                            "failed to decode address from burn memo {:?}. Skipping withdrawal.",
                            memo.0
                        );
                        None
                    }
                })
                .collect(),
            block: res.block,
        }
    }
}

impl<S> Service<Request> for CarteZcashService<S>
where
    S: Service<
            tiny_cash::service::Request,
            Response = tiny_cash::service::Response,
            Error = BoxError,
        > + Send
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
                        .call(tiny_cash::service::Request::Mint {
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
                        .call(tiny_cash::service::Request::IncludeTransaction { transaction: txn })
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
