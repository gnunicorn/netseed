# 1: Build the exe
FROM rust:1.42 as builder
WORKDIR /usr/src

# 1a: Prepare for static linking
RUN apt-get update && \
    apt-get dist-upgrade -y && \
    apt-get install -y musl-tools && \
    rustup target add x86_64-unknown-linux-musl

# 1b: Download and compile Rust dependencies (and store as a separate Docker layer)
WORKDIR /usr/src/
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release --target x86_64-unknown-linux-musl

# 2: Copy the exe to an empty Docker image
FROM scratch
MAINTAINER Benjamin Kampmann <ben@parity.io>
COPY --from=builder /usr/src/target/release/netseed .
USER 1000
CMD [ "./netseed" ]