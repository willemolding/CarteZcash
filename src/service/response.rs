use json::object;
#[derive(Debug)]
pub enum Response {
    Accept { burned: u64 },
    Reject,
}

impl Response {
    pub fn host_request(&self, server_addr: &str) -> hyper::Request<hyper::Body> {
        let msg = match self {
            Response::Accept { .. } => object! { "status" => "accept" },
            Response::Reject => object! {"status" => "reject"},
        };

        hyper::Request::builder()
            .method(hyper::Method::POST)
            .header(hyper::header::CONTENT_TYPE, "application/json")
            .uri(format!("{}/finish", server_addr))
            .body(hyper::Body::from(msg.dump()))
            .unwrap()
    }

    // the response may optionally include a voucher request to send to the host
    // e.g. if some coins were burned and some need to be released on L1
    pub fn voucher_request(
        &self,
        server_addr: &str,
        dest: ethereum_types::Address,
        value: ethereum_types::U256,
    ) -> Option<hyper::Request<hyper::Body>> {
        let response = object! {
            destination: format!("0x{}", hex::encode(dest.as_fixed_bytes())),
            payload: format!("0x{}", hex::encode(ethabi::encode(&[ethabi::Token::Uint(value)]))),
        };
        println!("Voucher request: {}", response.dump());
        Some(
            hyper::Request::builder()
                .method(hyper::Method::POST)
                .header(hyper::header::CONTENT_TYPE, "application/json")
                .uri(format!("{}/voucher", server_addr))
                .body(hyper::Body::from(response.dump()))
                .unwrap(),
        )
    }
}
