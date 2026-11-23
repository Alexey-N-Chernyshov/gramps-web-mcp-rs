# Contributing

## Local build

```bash
cp .env.example .env
# edit .env with your values
cargo run
```

With `MCP_OAUTH_ENABLED=true`, also set `MCP_OAUTH_STORAGE_PATH=./oauth_store.json` —
the default `/data/oauth_store.json` only exists inside the Docker image.

## Docker build

```bash
docker compose -f docker-compose.dev.yml build
docker compose -f docker-compose.dev.yml up
```

## Tests

```bash
cargo test
```

Integration tests share one real Gramps Web instance via Docker Compose (see
`docker-compose.test.yml`), so Docker must be running. The first test to run
in a given `cargo test` invocation starts it and leaves it running — a
second local run reuses it instead of paying the startup cost again. Each
test gets its own isolation via a fresh Gramps Web tree, not a fresh
container. Set `GRAMPS_TEST_URL` to point at an already-running instance
instead (used by CI); `GRAMPSWEB_TREE: testdb` (single-tree) mode is only
covered by the one smoke test in `tests/singletree_smoke_test.rs`, which
always starts its own separate, throwaway instance.

## Lint

```bash
cargo fmt --check
cargo clippy --all-targets
```
