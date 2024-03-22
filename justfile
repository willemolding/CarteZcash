set positional-arguments

build:
    sunodo build


run-local:
    ROLLUP_HTTP_SERVER_URL=http://127.0.0.1:8080/host-runner cargo run --target aarch64-apple-darwin

sunodo-nobackend:
    sunodo run --no-backend

@deposit address amount:
    sunodo send ether --execLayerData=$1 --amount=$2 --rpc-url=http://127.0.0.1:8545 --chain-id=31337 --dapp=0x70ac08179605AF2D9e75782b8DEcDD3c22aA4D0C

@send txn_hex:
    sunodo send generic --input=$1 --rpc-url=http://127.0.0.1:8545 --chain-id=31337 --dapp=0x70ac08179605AF2D9e75782b8DEcDD3c22aA4D0C

check:
    cargo check --target aarch64-apple-darwin

test:
    cargo test --target aarch64-apple-darwin
