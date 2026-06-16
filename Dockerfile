FROM rust:1-slim-bookworm AS chef
RUN cargo install cargo-chef --locked
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim AS runtime

# ca-certificates needed for HTTPS to Gramps Web API
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -m -u 1000 gramps
USER gramps

COPY --from=builder /app/target/release/gramps-mcp-rs /usr/local/bin/gramps-mcp-rs

ENTRYPOINT ["gramps-mcp-rs"]
