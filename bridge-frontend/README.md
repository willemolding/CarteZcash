# CarteZcash Bridge Frontend

Forked from https://github.com/Mugen-Builders/frontend-cartesi-wallet-x

## Configurtion

Edit src/config.json to set the testnet parameters and deployment, inspect, graphql, rpc addresses.

## Available Scripts

In the project directory, run:

```shell
yarn
yarn codegen
```

to build the app.

```shell
yarn start
```

Runs the app in the development mode.\
Open [http://localhost:3000](http://localhost:3000) to view it in the browser.

## Voucher Notes

To execute Vouchers, the voucher epoch must be finalized so the rollups framework generate the proofs.
As a reminder, you can advance time in hardhat with the command:

```shell
curl --data '{"id":1337,"jsonrpc":"2.0","method":"evm_increaseTime","params":[864010]}' http://localhost:8545
```
Alternatively, you can run cartesi node with shorter epoch duration in seconds
```
cartesi run --epoch-duration=60
```
