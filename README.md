# CarteZcash

CarteZcash is a Zcash application specific Cartesi rollup. Created for the 2024 Cartesi Hackathon.

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
