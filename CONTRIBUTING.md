# Contributing

## Local build

```bash
cp .env.example .env
# edit .env with your values
cargo run
```

## Docker build

```bash
docker compose -f docker-compose.dev.yml build
docker compose -f docker-compose.dev.yml up
```

## Tests

```bash
cargo test
```

Integration tests spin up a real Gramps Web instance via Docker Compose
(see `docker-compose.test.yml`), so Docker must be running.

## Lint

```bash
cargo fmt --check
cargo clippy --all-targets
```
