# CarteZcash

CarteZcash takes Zcash and turns it into a Cartesi RollApp. Created for the 2024 Cartesi Hackathon.

## How it works

CarteZcash takes parts of the [Zebra](https://github.com/ZcashFoundation/zebra) Zcash client and uses them to build a mini version of the protocol called TinyCash. This is similar to how Optimism uses [minigeth](https://github.com/ethereum-optimism/minigeth) for the Ethereum state transition.

TinyCash is ZCash but with the following changes:

- All network upgrades up to NU5 are applied in the first block
- Each block contains only a single coinbase transaction and up to one user transaction
- No checking of proof-of-work
- Only a single fork allowed
- UTXOs cannot be verified out-of-order
- No miner rewards
- Transaction fees not enforced

TinyCash then runs inside the Cartesi machine to produce a fully functional rollup.

All interaction with the L2 is done via the Cartesi contracts. Deposits, transactions and withdrawals are all supported.

### Deposits

Using Portals it is possible to deposit Eth into the rollup and have it minted as CarteZcash coins into a transparent Zcash address.

This works by using the coinbase transaction functionality that was previously used for issuing mining rewards. Upon receiving an AdvanceState message that matches an Eth deposit action CarteZcash instructs TinyCash to mine a new block with a coinbase that mints coins to the wallet address decoded from the `execLayerData` field. These new minted coins are public (not shielded) but can be made anonymous by making another transaction into the shielded pool.

### Transfers

CarteZcash is able to process regular ZCash transactions produced and signed by any ZCash wallet. This includes private shielded transactions! 

The prepared transactions just need to be serialized and then sent to CarteZcash via the InputBox contract.

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
