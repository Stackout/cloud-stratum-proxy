FROM rust:1.42.0-slim AS build

# Add musl deps for targeting alpine build:
RUN apt-get update \
  && apt-get install musl-tools curl g++ make pkg-config libssl-dev openssl -y \
  && rustup target add x86_64-unknown-linux-musl

ENV name=cloud-stratum-proxy
RUN USER=root cargo new --bin build --name cloud-stratum-proxy
WORKDIR /build

ENV CC musl-gcc
ENV PREFIX /usr/local
ENV PATH /usr/local/bin:$PATH
ENV PKG_CONFIG_PATH /usr/local/lib/pkgconfig
ENV OPENSSL_LIB_DIR $PREFIX/lib
ENV OPENSSL_INCLUDE_DIR $PREFIX/include
ENV OPENSSL_DIR $PREFIX
ENV OPENSSL_STATIC 1
ENV PKG_CONFIG_ALLOW_CROSS 1
ENV SSL_VER 1.0.2o
RUN curl -sL http://www.openssl.org/source/openssl-$SSL_VER.tar.gz | tar xz \
    &&  cd openssl-$SSL_VER \
    &&  ./Configure no-shared --prefix=$PREFIX --openssldir=$PREFIX/ssl no-zlib linux-x86_64 -fPIC \
    &&  make -j$(nproc) && make install && cd .. && rm -rf openssl-$SSL_VER

RUN pkg-config --cflags openssl

# Faster images builds by first building deps.
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml
RUN RUSTFLAGS=-Clinker=musl-gcc cargo build \
    --release \
    --target=x86_64-unknown-linux-musl \
  && rm ./src/*.rs

# Build source.
COPY ./src ./src
RUN cd ./src/ && cat main.rs
RUN RUSTFLAGS=-Clinker=musl-gcc cargo build \
    --release \
    --target=x86_64-unknown-linux-musl

FROM alpine:3.11.5

# Run as non-root user.
RUN addgroup -g 1000 cloud-stratum-proxy \
    && adduser -D -s /bin/sh -u 1000 -G cloud-stratum-proxy cloud-stratum-proxy
USER cloud-stratum-proxy

# Copy binary from build stage.
COPY --chown=cloud-stratum-proxy --from=build /build/target/x86_64-unknown-linux-musl/release/cloud-stratum-proxy /usr/local/bin/cloud-stratum-proxy

CMD ["cloud-stratum-proxy"]
