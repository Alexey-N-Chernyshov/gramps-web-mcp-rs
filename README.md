# gramps-web-mcp-rs

MCP server for [Gramps Web](https://www.grampsweb.org/) genealogy API, written in Rust.

Connects your AI client (Claude Desktop, Claude Code, etc.) directly to your Gramps Web instance. Exposes tools covering search, read, create, update, delete, merge, and transaction operations.

## Setup

Choose a deployment mode based on where you run the MCP server:

| Mode                        | When to use                                               | Guide                                                          |
| --------------------------- | --------------------------------------------------------- | -------------------------------------------------------------- |
| **Local (stdio)**           | AI client and Gramps Web are on the same machine          | [docs/setup-local.md](docs/setup-local.md)                     |
| **LAN / VPN (HTTP)**        | Gramps Web is on a home server, reachable via LAN or VPN  | [docs/setup-server-lan.md](docs/setup-server-lan.md)           |
| **Public server (HTTP+TLS)**| Gramps Web is exposed to the internet via reverse proxy   | [docs/setup-server-public.md](docs/setup-server-public.md)     |

## Configuration

| Variable            | Default   | Description                                                                                                                                                                                                                      |
| ------------------- | --------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `GRAMPS_API_URL`    | —         | Base URL of your Gramps Web instance, e.g. `https://gramps.example.com` or `http://localhost:5000`. Must include the scheme (`http://` or `https://`).                                                                           |
| `GRAMPS_USERNAME`   | —         | Gramps Web username                                                                                                                                                                                                              |
| `GRAMPS_PASSWORD`   | —         | Gramps Web password                                                                                                                                                                                                              |
| `GRAMPS_READONLY`   | `false`   | Set to `true` to expose only read tools. Convenience flag — for real access control use a Gramps Web account with the Viewer role.                                                                                               |
| `MCP_TRANSPORT`     | `stdio`   | Transport mode: `stdio` or `http`                                                                                                                                                                                                |
| `MCP_HTTP_HOST`     | `0.0.0.0` | Bind host in HTTP mode                                                                                                                                                                                                           |
| `MCP_HTTP_PORT`     | `3000`    | Bind port in HTTP mode                                                                                                                                                                                                           |
| `MCP_AUTH_TOKEN`    | —         | Bearer token protecting the `/mcp` endpoint in HTTP mode. If unset, no authentication is performed — only safe on a trusted LAN.                                                                                                 |
| `MCP_ALLOWED_HOSTS` | —         | Comma-separated list of allowed `Host` header values for HTTP transport (DNS-rebinding protection). If unset, only `localhost`/`127.0.0.1`/`::1` are allowed (rmcp default). Example: `your-domain.com:8888,localhost,127.0.0.1` |

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
