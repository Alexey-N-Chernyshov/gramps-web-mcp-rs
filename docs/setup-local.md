# Local setup (stdio)

The MCP server runs as a child process of the AI client and communicates over stdin/stdout. No network port is opened, no auth token needed. The simplest setup.

**Prerequisites:** 
 - running [Gramps Web](https://www.grampsweb.org/) instance
 - [Docker](https://docs.docker.com/get-docker/) installed on the client machine

## Step 1 — Create a Gramps Web user for MCP

Create a dedicated account the MCP server will use to authenticate:

1. Open Gramps Web → **Settings** → **User administration** → **Add user**
2. Set a username and password
3. Set the role:
   - **Editor** — full read/write access (recommended)
   - **Viewer** — read-only; pair with `GRAMPS_READONLY=true` to also hide write tools from the AI

## Step 2 — Configure your AI client

The Docker image is pulled automatically on first use — no `docker pull` needed.

### Claude Desktop

Edit `~/Library/Application Support/Claude/claude_desktop_config.json` (macOS) or `%APPDATA%\Claude\claude_desktop_config.json` (Windows):

```json
{
  "mcpServers": {
    "gramps-web": {
      "command": "docker",
      "args": [
        "run", "--rm", "-i",
        "-e", "GRAMPS_API_URL",
        "-e", "GRAMPS_USERNAME",
        "-e", "GRAMPS_PASSWORD",
        "-e", "GRAMPS_READONLY",
        "ghcr.io/alexey-n-chernyshov/gramps-web-mcp-rs:latest"
      ],
      "env": {
        "GRAMPS_API_URL": "https://gramps.example.com",
        "GRAMPS_USERNAME": "your-mcp-username",
        "GRAMPS_PASSWORD": "your-mcp-password"
      }
    }
  }
}
```

Restart Claude Desktop. The gramps-web server should appear in the tools menu.

### Claude Code

Add to `.mcp.json` in your project directory:

```json
{
  "mcpServers": {
    "gramps-web": {
      "command": "docker",
      "args": [
        "run", "--rm", "-i",
        "-e", "GRAMPS_API_URL=https://gramps.example.com",
        "-e", "GRAMPS_USERNAME=your-mcp-username",
        "-e", "GRAMPS_PASSWORD=your-mcp-password",
        "ghcr.io/alexey-n-chernyshov/gramps-web-mcp-rs:latest"
      ]
    }
  }
}
```

Or via CLI (adds to the project `.mcp.json`):

```bash
claude mcp add --scope project gramps-web \
  docker -- run --rm -i \
  -e GRAMPS_API_URL=https://gramps.example.com \
  -e GRAMPS_USERNAME=your-mcp-username \
  -e GRAMPS_PASSWORD=your-mcp-password \
  ghcr.io/alexey-n-chernyshov/gramps-web-mcp-rs:latest
```

## Keeping up to date

```bash
docker pull ghcr.io/alexey-n-chernyshov/gramps-web-mcp-rs:latest
```
