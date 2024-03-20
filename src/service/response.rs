use json::object;
pub enum Response {
    Accept { burned: u64 },
    Reject,
}

impl Response {
    pub fn host_request(&self, server_addr: &str) -> hyper::Request<hyper::Body> {
        let msg = match self {
            Response::Accept{..} => object! { "status" => "accept" },
            Response::Reject => object! {"status" => "reject"},
        };

        hyper::Request::builder()
            .method(hyper::Method::POST)
            .header(hyper::header::CONTENT_TYPE, "application/json")
            .uri(format!("{}/finish", server_addr))
            .body(hyper::Body::from(msg.dump())).unwrap()
    }
}
