FROM rust:1-slim-bookworm AS chef
ARG TARGETARCH
RUN apt-get update && apt-get install -y --no-install-recommends musl-tools musl-dev && \
    rm -rf /var/lib/apt/lists/* && \
    case "$TARGETARCH" in \
        amd64) rustup target add x86_64-unknown-linux-musl ;; \
        arm64) rustup target add aarch64-unknown-linux-musl ;; \
    esac && \
    cargo install cargo-chef --locked && \
    cargo install cargo-about --features cli --locked
WORKDIR /app
ENV CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER=musl-gcc \
    CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER=musl-gcc \
    CC_x86_64_unknown_linux_musl=musl-gcc \
    CC_aarch64_unknown_linux_musl=musl-gcc \
    RUSTFLAGS="-C target-feature=+crt-static"

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

FROM gcr.io/distroless/static-debian12:nonroot AS runtime
COPY --from=builder /gramps-web-mcp-rs /usr/local/bin/gramps-web-mcp-rs
COPY --from=builder /app/THIRD_PARTY_NOTICES.html /THIRD_PARTY_NOTICES.html
ENTRYPOINT ["/usr/local/bin/gramps-web-mcp-rs"]
