# Server-side setup (HTTP)

The MCP server runs alongside Gramps Web on a home server or VPS and exposes an HTTP endpoint. The AI client connects over the network. No Docker required on the client machine.

**Prerequisites:**

- Gramps Web running via Docker Compose
- access to its `docker-compose.yml`

## Step 1 — Create a Gramps Web user for MCP

Create a dedicated account the MCP server will use to authenticate:

1. Open Gramps Web → **Settings** → **User administration** → **Add user**
2. Set a username and password
3. Set the role:
   - **Editor** — full read/write access (recommended)
   - **Viewer** — read-only; pair with `GRAMPS_READONLY=true` to also hide write tools from the AI

## Step 2 — Generate an auth token

The MCP endpoint will be reachable over the network — protect it with a secret token:

```bash
openssl rand -hex 32
```

Save the output. You'll need it in both the server config (Step 3) and the client config (Step 4).

## Step 3 — Add the MCP service to docker-compose

In your Gramps Web `docker-compose.yml`, add:

```yaml
gramps-mcp:
  image: ghcr.io/alexey-n-chernyshov/gramps-web-mcp-rs:latest
  restart: unless-stopped
  environment:
    GRAMPS_API_URL: http://grampsweb:5000 # internal Docker service name and port
    GRAMPS_USERNAME: your-mcp-username
    GRAMPS_PASSWORD: your-mcp-password
    MCP_TRANSPORT: http
    MCP_HTTP_PORT: 3000
    MCP_AUTH_TOKEN: your-secret-token
  ports:
    - "3000:3000"
```

> `GRAMPS_API_URL` must use the Docker service name (e.g. `gramps-web`), not `localhost` — containers resolve each other by service name within the same Compose network.

Start the service:

```bash
docker compose up -d gramps-mcp
```

Verify it's running:

```bash
curl http://localhost:3000/health
# → ok
```

## Step 4 — Configure your AI client

Replace `your-server-ip` with your server's IP address or domain name, and `your-secret-token` with the token from Step 2.

### Claude Desktop

```json
{
  "mcpServers": {
    "gramps-web": {
      "type": "http",
      "url": "http://your-server-ip:3000/mcp",
      "headers": {
        "Authorization": "Bearer your-secret-token"
      }
    }
  }
}
```

### Claude Code

Via CLI:

```bash
claude mcp add --transport http --scope project \
  gramps-web http://your-server-ip:3000/mcp \
  -H "Authorization: Bearer your-secret-token"
```

Or add to `.mcp.json` in your project directory:

```json
{
  "mcpServers": {
    "gramps-web": {
      "type": "http",
      "url": "http://your-server-ip:3000/mcp",
      "headers": {
        "Authorization": "Bearer your-secret-token"
      }
    }
  }
}
```

## Keeping up to date

```bash
docker compose pull gramps-mcp && docker compose up -d gramps-mcp
```
