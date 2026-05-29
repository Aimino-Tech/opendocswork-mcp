# Stage 1: Build musl binary
FROM rust:1.85-alpine3.21 AS builder
RUN apk add --no-cache musl-dev
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY vendor/ vendor/
COPY src/ src/
RUN cargo build --release --target x86_64-unknown-linux-musl

# Stage 2: Minimal runtime (scratch)
FROM scratch
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/office-oxide-mcp /office-oxide-mcp
ENTRYPOINT ["/office-oxide-mcp"]
