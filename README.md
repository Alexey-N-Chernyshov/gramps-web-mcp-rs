# gramps-mcp-rs

MCP server for [Gramps Web](https://www.grampsweb.org/) genealogy API, written in Rust.

## For users

### Prerequisites

- Running [Gramps Web](https://www.grampsweb.org/) instance
- [Docker](https://docs.docker.com/get-docker/)

### Setup

**1. Clone and build the image**

```bash
git clone https://github.com/YOUR_USERNAME/gramps-mcp-rs.git
cd gramps-mcp-rs
docker compose -f docker-compose.dev.yml build
```

**2. Configure your MCP client**

For Claude Desktop, add to `~/Library/Application Support/Claude/claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "gramps": {
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
        "gramps-mcp-rs:dev"
      ],
      "env": {
        "GRAMPS_API_URL": "https://gramps.example.com",
        "GRAMPS_USERNAME": "your-username",
        "GRAMPS_PASSWORD": "your-password"
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
docker compose -f docker-compose.dev.yml run --rm gramps-mcp
```
