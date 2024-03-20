# CarteZcash

CarteZcash is a Zcash application specific Cartesi rollup. Created for the 2024 Cartesi Hackathon.

## How it works

CarteZcash takes parts of the Rust ZCash client [Zebra](https://github.com/ZcashFoundation/zebra) and uses it to build a mini version of the protocol called TinyCash. TinyCash makes the following changes to the ZCash protocol:

- Each block contains only a single coinbase transaction and one optional user transaction
- No proof-of-work checks are performed
- A fixed number of coins is not mined each block
- All network upgrades up to NU5 are applied in the first block

TinyCash runs inside the Cartesi machine and adds new blocks/transaction when requested.

### Deposits

Using Portals it is possible to deposit Eth into the rollup and have it minted as CarteZcash coins into a transparent Zcash address.

This works by using the coinbase transaction functionality that was previously used for issuing mining rewards. Upon receiving an AdvanceState message that matches an Eth deposit action CarteZcash instructs TinyCash to mine a new block with a coinbase that mints coins to the wallet address decoded from the `execLayerData` field. These new minted coins are public (not shielded) but can be made anonomoyus by making another transaction into the shielded pool.

### Transfers

CarteZcash is able to process regular ZCash transactions produced by a ZCash wallet. This includes private shielded transactions!! 

### Withdrawals

To withdraw from the CarteZcash L2 and get your coins back on L1 you simply cast your coins into the fires of Mt Doom! What this means is you make a transparent transaction sending your coins to the Mt Doom address. This address contains the null script so these coins can never be spent again. CarteZcash watches for transaction to this address and when it observes one will issue a voucher to release the corresponding number of coins on L1.

## Building

The project uses [just](https://github.com/casey/just) as a command runner. Please install that first.

Build with:

```shell
just build
```

This cross-compiles for risvc using docker.

Check with:

```shell
just check
```

This does a native build without docker and should be much quicker for checking while developing.
