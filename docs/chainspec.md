# CarteZcash Chain Speciation

This document outlines the changes needed to Zcash wallets and other infrastructure to support various different instances of CarteZcash. 

Unlike ZCash, CarteZcash is not a singleton, there can be multiple instances deployed to support different assets (e.g. shielded Eth, Shielded DAI). These need to be distinguished from each other so that signatures cannot be reused between instances.

## Common

Configurations shared by all instances

### Shielded Pools

CarteZcash only supports the Orchard shielded pool. Any other shielded pool transactions will be rejected.

When querying the state trees from the full-node RPC it will return an empty tree for all but the Orchard state tree.

### Network upgrade activation heights

Wallets typically hard-code the activation heights for the various Zcash upgrades for the mainnet and testnet. These are the upgrade heights for all CarteZcash instances:

```
Genesis: 0
BeforeOverwinter: 1
Overwinter: 1
Sapling: 1
Blossom: 1
Heartwood: 1
Canopy: 1
Nu5: 1
```

## Instance Specific

### Chain Name

Each instance must have a unique name which identifies: 

- the base chain
- the asset the protocol is instantiated for
- if it is a testnet

The proposed chain naming convention is:

`<?testnet>.<asset-ID>.<base-chainID>`

For example

```shell
eth.1 // eth on mainnet ethereum
usdt.10 // USDT on Optimism
testnet.dai.10 // a CarteZcash testnet for DAI on Optimism
testnet.eth.421614 // a CarteZcash testnet for Eth on the Arbitrum Sepolia testnet
```

### RPC

#### URL

By convention CarteZcash full nodes SHOULD serve their RPC on a subdomain following the chain name

e.g.

```
eth.1.cartezcash.io
testnet.dai.10.cartezcash.io
```

This is the main way wallet users will configure their local wallets to connect to a particular instance.

#### GetLightdInfo

When calling the RPC method `GetLightdInfo` the full-node MUST use the `chainName` field following the same convention as above.
 
The remainder of the fields should follow the spec as described in the lightwalletd RPC

```
 {
    "version": "v0.1.0",
    "vendor": "CarteZcash",
    "taddrSupport": true,
    "chainName": "eth.1", <-------------------
    "saplingActivationHeight": "1",
    "consensusBranchId": "c2d6d0b4",
    "blockHeight": "123",
    "gitCommit": "f7795c83a397dcb25fc2779537308fb91e1bc99d",
    "branch": "",
    "buildDate": "2024-01-01",
    "buildUser": "wollum",
    "estimatedHeight": "123",
    "zcashdBuild": "N/A",
    "zcashdSubversion": "N/A"
}
```
