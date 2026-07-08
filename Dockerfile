FROM rust:1-alpine AS chef
ARG TARGETARCH
RUN apk add --no-cache gcc musl-dev && \
    case "$TARGETARCH" in \
        amd64) rustup target add x86_64-unknown-linux-musl ;; \
        arm64) rustup target add aarch64-unknown-linux-musl ;; \
    esac && \
    cargo install cargo-chef --locked && \
    cargo install cargo-about --features cli --locked
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
ARG TARGETARCH
COPY --from=planner /app/recipe.json recipe.json
RUN case "$TARGETARCH" in \
        amd64) RUST_TARGET=x86_64-unknown-linux-musl ;; \
        arm64) RUST_TARGET=aarch64-unknown-linux-musl ;; \
    esac && \
    cargo chef cook --release --target $RUST_TARGET --recipe-path recipe.json
COPY . .
RUN case "$TARGETARCH" in \
        amd64) RUST_TARGET=x86_64-unknown-linux-musl ;; \
        arm64) RUST_TARGET=aarch64-unknown-linux-musl ;; \
    esac && \
    cargo build --release --target $RUST_TARGET && \
    cp target/$RUST_TARGET/release/gramps-web-mcp-rs /gramps-web-mcp-rs && \
    cargo about generate -c .config/about.toml .config/about.hbs -o THIRD_PARTY_NOTICES.html

FROM alpine:3.24 AS runtime
RUN addgroup -S nonroot && adduser -S nonroot -G nonroot
COPY --from=builder /gramps-web-mcp-rs /usr/local/bin/gramps-web-mcp-rs
COPY --from=builder /app/THIRD_PARTY_NOTICES.html /THIRD_PARTY_NOTICES.html
USER nonroot
ENTRYPOINT ["/usr/local/bin/gramps-web-mcp-rs"]
