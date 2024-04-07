set positional-arguments

build:
    sunodo build

run:
    sunodo run --epoch-duration=10

run-local:
    ROLLUP_HTTP_SERVER_URL=http://127.0.0.1:8080/host-runner GRPC_SERVER_URL="[::1]:50051" cargo run

run-proxy:
    CARTESI_NODE_URL="0.0.0.0:8080" cargo run -p cartezcash-proxy

sunodo-nobackend:
    sunodo run --no-backend

##### Interact with dApp via sunodo

@deposit address amount:
    sunodo send ether --execLayerData=$1 --amount=$2 --rpc-url=http://127.0.0.1:8545 --chain-id=31337 --dapp=0x70ac08179605AF2D9e75782b8DEcDD3c22aA4D0C

@send txn_hex:
   just withdraw 0x0000000000000000000000000000000000000000 $1

@withdraw address txn_hex:
    sunodo send generic --input="$1$2" --rpc-url=http://127.0.0.1:8545 --chain-id=31337 --dapp=0x70ac08179605AF2D9e75782b8DEcDD3c22aA4D0C

execute_voucher:
    cast send 0x70ac08179605AF2D9e75782b8DEcDD3c22aA4D0C "executeVoucher(address, bytes, struct Proof _proof)"

##### wallet related commands

install-wallet:
    cargo install --git https://github.com/willemolding/zingolib --branch willem/tinycash --bin zingo-cli

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
