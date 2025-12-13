FROM rust:1.91-slim AS builder

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libpq-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations

RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libpq5 \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/cargo-hold /usr/local/bin/cargo-hold

ENV RUST_LOG=info
ENV PUBLIC_HOST=0.0.0.0
ENV PUBLIC_PORT=8080
ENV PRIVATE_HOST=0.0.0.0
ENV PRIVATE_PORT=8081

EXPOSE 8080 8081

CMD ["cargo-hold"]
