use std::env;
use tower::Service;

use service::{Request, Response, CarteZcashService};

mod service;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let client = hyper::Client::new();
    let server_addr = env::var("ROLLUP_HTTP_SERVER_URL")?;
    let mut service = CarteZcashService;

    let mut response = Response::Accept { burned: 0 };
    loop {
        println!("Sending finish");
        let response = client.request(response.host_request(&server_addr)).await?;
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

            let response = service.call(dAppRequest).await?;
        }
    }
}
