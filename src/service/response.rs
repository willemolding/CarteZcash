use json::object;

#[derive(Clone, Debug)]
pub enum Response {
    Accept {
        burned: u64,
    },
    Report {
        payload: Vec<u8>,
    },
    #[allow(dead_code)]
    Reject,
}

impl Response {
    pub fn host_request(&self, server_addr: &str) -> hyper::Request<hyper::Body> {
        let (route, body) = match self {
            Response::Accept { .. } | Response::Report { .. } => {
                ("finish", object! { "status" => "accept" })
            }
            Response::Reject => ("finish", object! {"status" => "reject"}),
        };

        let request = hyper::Request::builder()
            .method(hyper::Method::POST)
            .header(hyper::header::CONTENT_TYPE, "application/json")
            .uri(format!("{}/{}", server_addr, route))
            .body(hyper::Body::from(body.dump()))
            .unwrap();
        request
    }

    // the response may optionally include a voucher request to send to the host
    // e.g. if some coins were burned and some need to be released on L1
    pub fn voucher_request(
        &self,
        server_addr: &str,
        dest: ethereum_types::Address,
        value: ethereum_types::U256,
    ) -> Option<hyper::Request<hyper::Body>> {
        match self {
            Response::Accept { burned } => {
                if *burned == 0 {
                    return None;
                }
            }
            Response::Reject => return None,
            Response::Report { .. } => return None,
        }

        let response = object! { // hack - dApp address is hard-coded for now
            destination: "0x70ac08179605AF2D9e75782b8DEcDD3c22aA4D0C",//format!("0x{}", hex::encode(dest.as_fixed_bytes())),
            payload: format!("0x{}", hex::encode(withdraw_ether_call(dest, value)))//format!("0x{}", hex::encode(ethabi::encode(&[ethabi::Token::Address(dest), ethabi::Token::Uint(value)]))),
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

    pub fn report_request(&self, server_addr: &str) -> Option<hyper::Request<hyper::Body>> {
        match self {
            Response::Accept { .. } | Response::Reject => None,
            Response::Report { payload } => {
                let response = object! {
                    payload: format!("0x{}", hex::encode(payload)),
                };
                Some(
                    hyper::Request::builder()
                        .method(hyper::Method::POST)
                        .header(hyper::header::CONTENT_TYPE, "application/json")
                        .uri(format!("{}/report", server_addr))
                        .body(hyper::Body::from(response.dump()))
                        .unwrap(),
                )
            }
        }
    }
}

fn withdraw_ether_call(receiver: ethereum_types::Address, value: ethereum_types::U256) -> Vec<u8> {
    let function = alloy_json_abi::Function::parse("withdrawEther(address,uint256)").unwrap();

    let encoded_params = ethabi::encode(&[
        ethabi::Token::Address(receiver),
        ethabi::Token::Uint(
            value
                .checked_mul(ethereum_types::U256::from(10_000_000_000_u64))
                .unwrap(),
        ),
    ]);

    let mut encoded = Vec::new();
    encoded.extend_from_slice(&function.selector().as_slice());
    encoded.extend_from_slice(&encoded_params);

    encoded
}
