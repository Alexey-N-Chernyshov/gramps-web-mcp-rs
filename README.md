# gramps-web-mcp-rs

MCP server for [Gramps Web](https://www.grampsweb.org/) genealogy API, written in Rust.

## For users

### Prerequisites

- Running [Gramps Web](https://www.grampsweb.org/) instance
- [Docker](https://docs.docker.com/get-docker/)

### Setup

**1. Pull the image**

```bash
curl -O https://raw.githubusercontent.com/Alexey-N-Chernyshov/gramps-web-mcp-rs/main/docker-compose.yml
docker compose pull
```

To update later, run `docker compose pull` again.

**2. Configure your MCP client**

For Claude Desktop, add to `~/Library/Application Support/Claude/claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "gramps-web": {
      "command": "docker",
      "args": [
        "run",
        "--rm",
        "-i",
        "-e",
        "GRAMPS_API_URL",
        "-e",
        "GRAMPS_USERNAME",
        "-e",
        "GRAMPS_PASSWORD",
        "-e",
        "GRAMPS_READONLY",
        "ghcr.io/alexey-n-chernyshov/gramps-web-mcp-rs:latest"
      ],
      "env": {
        "GRAMPS_API_URL": "https://gramps.example.com",
        "GRAMPS_USERNAME": "your-username",
        "GRAMPS_PASSWORD": "your-password",
        "GRAMPS_READONLY": "false"
      }
    }
  }
}
```

### Configuration

| Variable          | Description                                               |
| ----------------- | --------------------------------------------------------- |
| `GRAMPS_API_URL`  | Base URL of Gramps Web, e.g. `https://gramps.example.com` |
| `GRAMPS_USERNAME` | Login username                                            |
| `GRAMPS_PASSWORD` | Login password                                            |
| `GRAMPS_READONLY` | Set to `true` to hide all write tools (default: `false`)  |

### Tools

67 tools across search, get, create, update, delete, merge, and transaction categories. See [docs/tools.md](docs/tools.md) for the full reference.

---

## For developers

### Local build

```bash
cp .env.example .env
# edit .env with your values

export $(grep -v '^#' .env | xargs)
cargo run --release
```

### Docker build

```bash
docker compose -f docker-compose.dev.yml build
```
