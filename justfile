set positional-arguments
set dotenv-load
set dotenv-path := ".env.local"

build:
    sunodo build

run:
    sunodo run --epoch-duration=10

run-local:
     cargo run --features lightwalletd

run-fullnode:
    cargo run --no-default-features --features listen-graphql,lightwalletd 

sunodo-nobackend:
    sunodo run --no-backend

##### Docker

build-fullnode-docker:
    docker build -f fullnode.Dockerfile -t cartezcash/fullnode:latest .

run-fullnode-docker:
    docker run -it --rm -p 50051:50051 -e ROLLUP_HTTP_SERVER_URL=http://host.docker.internal:8080/graphql -e GRPC_SERVER_URL="[::1]:50051" cartezcash/fullnode:latest

##### Interact with dApp via sunodo

@deposit address amount:
    sunodo send ether --execLayerData=$1 --amount=$2 --rpc-url=http://127.0.0.1:8545 --chain-id=31337 --dapp=0xab7528bb862fB57E8A2BCd567a2e929a0Be56a5e

@send txn_hex:
    sunodo send generic --input="0x$2" --rpc-url=http://127.0.0.1:8545 --chain-id=31337 --dapp=0xab7528bb862fB57E8A2BCd567a2e929a0Be56a5e

execute_voucher:
    cast send 0xab7528bb862fB57E8A2BCd567a2e929a0Be56a5e "executeVoucher(address, bytes, struct Proof _proof)"

send_address:
    sunodo send dapp-address --rpc-url=http://127.0.0.1:8545 --chain-id=31337 --dapp=0xab7528bb862fB57E8A2BCd567a2e929a0Be56a5e

##### wallet related commands

install-wallet:
    cargo install --git https://github.com/willemolding/zingolib --branch willem/tinycash-desktop --bin zingo-cli

## Do not use these wallets on mainnet!

start-wallet:
    zingo-cli --data-dir ./walletdata --server localhost:50051 --from "wood unaware body couch morning flavor wage relax inject point scare firm emotion civil risk athlete asthma pave mango title spatial celery use modify" --birthday 0
restart-wallet:
    zingo-cli --data-dir ./walletdata --server localhost:50051

start-wallet-2:
    zingo-cli --data-dir ./walletdata2 --server localhost:50051 --from "february day pink knee nut struggle poem silver hawk voice stay rule food cabbage eight phrase parent spider forget laundry wagon dwarf improve flee" --birthday 0
restart-wallet-2:
    zingo-cli --data-dir ./walletdata --server localhost:50051

clear-wallet:
    rm -rf ./walletdata*
