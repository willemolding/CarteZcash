# Deployment Procedure

There are 4 main components that need to be updated on a new release. These are:
- The on-chain dApp registration
- The dApp node
- The fullnode service
- The bridge UI

Both the dApp node and the fullnode are hosted on flly.io. 

## On-chain Deployment

This needs to be done first as other components depend on the result.

First step is to build the dApp image

```shell
cartesi build
cartesi hash
```

Copy the templateHash that is printed out.

next visit

```
https://sunodo.io/deploy?templateHash=<template-hash>
```
and select the chain to deploy to and provide the wallet address of the dApp operator.

Make the deployment transaction and copy the resulting variables show into the [fly.toml](../fly.toml) file. Also update the Dapp address in the bridge-frontent [config.json](../bridge-frontend/src/config.json).

## Hosted Deployments (via Github actions)

- Commit the variables added to the fly.toml
- push to the repo and merge to main
- Cut a new release using https://github.com/willemolding/CarteZcash/releases/new

This will automatically build the docker images required for the Cartesi node and the full node, as well as deploy the latest bridge UI.

After this has completed manually trigger the fly.io service to redeploy from the fresh docker images by visiting https://github.com/willemolding/CarteZcash/actions/workflows/fly.yml and running the workflow.
(This may be automated in the future)

Wait a few minutes and it should be ready to go!

Wallets can sync using https://cartezcash-fullnode.fly.dev:443 as their lightwalletd GRPC provider
Deposit funds on the bridge at https://willemolding.github.io/CarteZcash/
