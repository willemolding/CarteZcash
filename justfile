set positional-arguments

build:
    sunodo build

run-local:
    ROLLUP_HTTP_SERVER_URL=http://127.0.0.1:8080/host-runner cargo run --release

run-proxy:
    cargo run -p cartezcash-proxy --release

sunodo-nobackend:
    sunodo run --no-backend

##### Interact with dApp via sunodo

@deposit address amount:
    sunodo send ether --execLayerData=$1 --amount=$2 --rpc-url=http://127.0.0.1:8545 --chain-id=31337 --dapp=0x70ac08179605AF2D9e75782b8DEcDD3c22aA4D0C

@send txn_hex:
   just withdraw 0x0000000000000000000000000000000000000000 $1

@withdraw address txn_hex:
    sunodo send generic --input="$1$2" --rpc-url=http://127.0.0.1:8545 --chain-id=31337 --dapp=0x70ac08179605AF2D9e75782b8DEcDD3c22aA4D0C


##### wallet related commands

install-wallet:
    cargo install --git https://github.com/willemolding/zingolib --branch willem/tinycash --bin zingo-cli

start-wallet:
    zingo-cli --data-dir ./walletdata --server localhost:50051

start-wallet-2:
    zingo-cli --data-dir ./walletdata2 --server localhost:50051

clear-wallet:
    rm -rf ./walletdata*
