# gramps-web-mcp-rs

MCP server for [Gramps Web](https://www.grampsweb.org/) genealogy API, written in Rust.

Connects your AI client (Claude Desktop, Claude Code, etc.) directly to your Gramps Web instance. Exposes tools covering search, read, create, update, delete, merge, and transaction operations.

## Setup

Choose a deployment mode based on where you run the MCP server:

| Mode | When to use | Guide |
|------|-------------|-------|
| **Local (stdio)** | AI client and Gramps Web are on the same machine | [docs/setup-local.md](docs/setup-local.md) |
| **Server-side (HTTP)** | Gramps Web runs on a home server or VPS | [docs/setup-server.md](docs/setup-server.md) |

## Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `GRAMPS_API_URL` | — | Base URL of your Gramps Web instance, e.g. `https://gramps.example.com` or `http://localhost:5000`. Must include the scheme (`http://` or `https://`). |
| `GRAMPS_USERNAME` | — | Gramps Web username |
| `GRAMPS_PASSWORD` | — | Gramps Web password |
| `GRAMPS_READONLY` | `false` | Set to `true` to expose only read tools. Convenience flag — for real access control use a Gramps Web account with the Viewer role. |
| `MCP_TRANSPORT` | `stdio` | Transport mode: `stdio` or `http` |
| `MCP_HTTP_HOST` | `0.0.0.0` | Bind host in HTTP mode |
| `MCP_HTTP_PORT` | `3000` | Bind port in HTTP mode |
| `MCP_AUTH_TOKEN` | — | Bearer token protecting the `/mcp` endpoint in HTTP mode. If unset, no authentication is performed — only safe on a trusted LAN. |

## Tools

67 tools across search, get, create, update, delete, merge, and transaction categories. See [docs/tools.md](docs/tools.md) for the full reference.

---

## For developers

### Local build

```bash
cp .env.example .env
# edit .env with your values
cargo run
```

### Docker build

```bash
docker compose -f docker-compose.dev.yml build
docker compose -f docker-compose.dev.yml up
```
