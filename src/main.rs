use service::{CarteZcashService, Request, Response};
use std::env;
use tower::{buffer::Buffer, util::BoxService, Service};

mod service;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let client = hyper::Client::new();
    let server_addr = env::var("ROLLUP_HTTP_SERVER_URL")?;

    // create our service with tiny_cash
    let mut cartezcash =
        CarteZcashService::new(Buffer::new(BoxService::new(tiny_cash::write::init()), 10));

    let mut status = Response::Accept { burned: 0 };
    loop {
        println!("Sending finish");
        let response = client.request(status.host_request(&server_addr)).await?;
        println!("Received finish status {}", response.status());

        if response.status() == hyper::StatusCode::ACCEPTED {
            println!("No pending rollup request, trying again");
        } else {
            let body = hyper::body::to_bytes(response).await?;
            let utf = std::str::from_utf8(&body)?;
            let req = json::parse(utf)?;
            println!("Received raw request: {:?}", req);
            let dAppRequest = Request::try_from(req)?;
            println!("Parsed request: {:?}", dAppRequest);

            status = cartezcash.call(dAppRequest).await?;
        }
    }
}
