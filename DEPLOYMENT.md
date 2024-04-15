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
sunodo build
```

Copy the last hash printed out. This is the templateHash

```shell
...
Manual yield rx-accepted (0x100000000 data)
Cycles: 566705956
566705956: 04003a8df136902507ea6103e4cd06e6e812b2ed25f630400ff3f19d0055aaec <---- templateHash
Storing machine: please wait
Successfully copied 386MB to /Users/willem/repos/CarteZcash/.sunodo/image
```

next visit

```
https://sunodo.io/deploy?templateHash=<template-hash>
```
and select the chain to deploy to and provide the wallet address of the dApp operator.

Make the deployment transaction and copy the resulting variables show into the [fly.toml](./fly.toml) file.

## Hosted Deployments (via Github actions)

- Commit the variables added to the fly.toml
- push to the repo and merge to main
- Cut a new release using https://github.com/willemolding/CarteZcash/releases/new

Once a new release has been tagged this will deploy the remaining components configured to work with the on-chain dApp.

Wait a few minutes and it should be ready to go!
