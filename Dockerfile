FROM lukemathwalker/cargo-chef:latest-rust-1-alpine AS chef
ARG TARGETARCH
RUN case "$TARGETARCH" in \
    amd64) rustup target add x86_64-unknown-linux-musl ;; \
    arm64) rustup target add aarch64-unknown-linux-musl ;; \
    esac
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
    cp target/$RUST_TARGET/release/gramps-web-mcp-rs /gramps-web-mcp-rs

FROM chef AS notices
ARG TARGETARCH
RUN case "$TARGETARCH" in \
    amd64) BINSTALL_ARCH=x86_64 ;; \
    arm64) BINSTALL_ARCH=aarch64 ;; \
    esac && \
    wget -qO- "https://github.com/cargo-bins/cargo-binstall/releases/latest/download/cargo-binstall-${BINSTALL_ARCH}-unknown-linux-musl.tgz" \
    | tar -xz -C /usr/local/cargo/bin && \
    CARGO_ABOUT_VERSION=$(wget -qO- https://api.github.com/repos/EmbarkStudios/cargo-about/releases/latest \
    | grep -m1 '"tag_name"' | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/') && \
    cargo binstall --no-confirm \
    --pkg-url "https://github.com/EmbarkStudios/cargo-about/releases/download/{version}/cargo-about-{version}-{target}.tar.gz" \
    --pkg-fmt tgz \
    --bin-dir "cargo-about-{version}-{target}/{bin}{binary-ext}" \
    "cargo-about@${CARGO_ABOUT_VERSION}"
COPY . .
RUN cargo about generate -c .config/about.toml .config/about.hbs -o THIRD_PARTY_NOTICES.html

FROM alpine:3.24 AS runtime
RUN addgroup -S nonroot && adduser -S nonroot -G nonroot
RUN mkdir /data && chown nonroot:nonroot /data
COPY --from=builder /gramps-web-mcp-rs /usr/local/bin/gramps-web-mcp-rs
COPY --from=notices /app/THIRD_PARTY_NOTICES.html /THIRD_PARTY_NOTICES.html
USER nonroot
ENTRYPOINT ["/usr/local/bin/gramps-web-mcp-rs"]
