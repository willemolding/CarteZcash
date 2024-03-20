use std::env;

use crate::request::Request;
use crate::response::Response;

mod request;
mod response;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let client = hyper::Client::new();
    let server_addr = env::var("ROLLUP_HTTP_SERVER_URL")?;

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
            let dAppRequest = Request::try_from(req)?;
            println!("Received request: {:?}", dAppRequest);

            // todo: convert request into response
        }
    }
}
