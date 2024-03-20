
build:
    sunodo build


run-local:
    ROLLUP_HTTP_SERVER_URL=http://127.0.0.1:8080/host-runner cargo run --target aarch64-apple-darwin

sunodo-nobackend:
    sunodo run --no-backend

check:
    cargo check --target aarch64-apple-darwin

test:
    cargo test --target aarch64-apple-darwin
