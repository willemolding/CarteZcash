# syntax=docker.io/docker/dockerfile:1
FROM ubuntu:22.04 as base

ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH \
    RUST_VERSION=1.76.0

ARG DEBIAN_FRONTEND=noninteractive
RUN <<EOF
set -e
apt update
apt install -y --no-install-recommends \
    build-essential=12.9ubuntu3 \
    ca-certificates=20230311ubuntu0.22.04.1 \
    wget=1.21.2-2ubuntu1 \
    libclang-dev \
    libssl-dev pkg-config
EOF
# libclang needed to build rocksdb. Can remove once this is no longer needed.
# libssl-dev and pkg-config needed to use reqwest

RUN set -eux; \
    dpkgArch="$(dpkg --print-architecture)"; \
    case "${dpkgArch##*-}" in \
    amd64) rustArch='x86_64-unknown-linux-gnu'; rustupSha256='0b2f6c8f85a3d02fde2efc0ced4657869d73fccfce59defb4e8d29233116e6db' ;; \
    armhf) rustArch='armv7-unknown-linux-gnueabihf'; rustupSha256='f21c44b01678c645d8fbba1e55e4180a01ac5af2d38bcbd14aa665e0d96ed69a' ;; \
    arm64) rustArch='aarch64-unknown-linux-gnu'; rustupSha256='673e336c81c65e6b16dcdede33f4cc9ed0f08bde1dbe7a935f113605292dc800' ;; \
    i386) rustArch='i686-unknown-linux-gnu'; rustupSha256='e7b0f47557c1afcd86939b118cbcf7fb95a5d1d917bdd355157b63ca00fc4333' ;; \
    *) echo >&2 "unsupported architecture: ${dpkgArch}"; exit 1 ;; \
    esac; \
    url="https://static.rust-lang.org/rustup/archive/1.26.0/${rustArch}/rustup-init"; \
    wget "$url"; \
    echo "${rustupSha256} *rustup-init" | sha256sum -c -; \
    chmod +x rustup-init; \
    ./rustup-init -y --no-modify-path --profile minimal --default-toolchain $RUST_VERSION --default-host ${rustArch}; \
    rm rustup-init; \
    chmod -R a+w $RUSTUP_HOME $CARGO_HOME; \
    rustup --version; \
    cargo --version; \
    rustc --version;

RUN cargo install cargo-chef
WORKDIR /opt/cartesi/cartezcash

###  the planner helps cache depdenency builds

FROM base AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

### Builder compiles the service

FROM base as builder
COPY --from=planner /opt/cartesi/cartezcash/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json --no-default-features --features listen-graphql,lightwalletd
COPY . .
RUN cargo build --release --no-default-features --features listen-graphql,lightwalletd

### final image is the dapp itself

FROM ubuntu:22.04

ENV PATH="/opt/cartesi/bin:/opt/cartesi/cartezcash:${PATH}"

WORKDIR /opt/cartesi/cartezcash
COPY --from=builder /opt/cartesi/cartezcash/target/release/cartezcash .

CMD ["cartezcash"]
