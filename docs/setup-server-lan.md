# LAN / VPN-only setup (HTTP, no TLS)

Use this when the MCP server only needs to be reachable from inside your home network, or over a VPN (Tailscale, WireGuard, etc.) — not from the public internet. No domain, no certificate, no reverse proxy.

> If the server needs to be reachable from the public internet, use [Server-side setup (HTTP) with a reverse proxy](setup-server-public.md) instead — plain HTTP without TLS is not safe outside a trusted network.

**Prerequisites:**

- Gramps Web running via Docker Compose
- access to its `docker-compose.yml`
- all clients are either on the same LAN, or connected via VPN

## Step 1 — Create a Gramps Web user for MCP

Create a dedicated account the MCP server will use to authenticate:

1. Open Gramps Web → **Settings** → **User administration** → **Add user**
2. Set a username and password
3. Set the role:
   - **Editor** — full read/write access (recommended)
   - **Viewer** — read-only; pair with `GRAMPS_READONLY=true` to also hide write tools from the AI

## Step 2 — Decide on an auth token

Even without TLS, anyone who can reach the port can use the server if it has no token — set one:

```bash
openssl rand -hex 32
```

If the network is genuinely single-user and fully trusted (e.g. a VPN with only your own devices), you can skip `MCP_AUTH_TOKEN` and rely on network-level access control alone. If in doubt, set it anyway — it costs nothing.

## Step 3 — Add the MCP service to docker-compose

```yaml
gramps-mcp:
  image: ghcr.io/alexey-n-chernyshov/gramps-web-mcp-rs:latest
  container_name: gramps-mcp
  restart: always
  environment:
    GRAMPS_API_URL: http://grampsweb:5000
    GRAMPS_USERNAME: your-mcp-username
    GRAMPS_PASSWORD: your-mcp-password
    MCP_TRANSPORT: http
    MCP_HTTP_PORT: 3000
    MCP_AUTH_TOKEN: your-secret-token
    MCP_ALLOWED_HOSTS: 192.168.1.50:3000 # required — see note below
  ports:
    - "3000:3000"
  networks:
    - default
```

> **Why `MCP_ALLOWED_HOSTS` is required even on a LAN:** the server's default DNS-rebinding protection only allows `localhost`/`127.0.0.1`/`::1`. As soon as you connect from another device — by LAN IP, hostname, or VPN address — the `Host` header is no longer `localhost`, and the request gets rejected by the MCP server itself unless that exact value is listed. Set it to whatever host:port your clients will actually use to connect (LAN IP, mDNS hostname, or your VPN-assigned address/hostname).

Start the service:

```bash
docker compose up -d gramps-mcp
```

Verify it's reachable from the server itself:

```bash
curl http://localhost:3000/health
# → ok
```

Then verify from a client machine on the LAN/VPN:

```bash
curl http://192.168.1.50:3000/health
```

If this fails while the local check succeeds, double-check your firewall (`ufw`, cloud security group, etc.) allows port 3000 from your LAN/VPN range.

### Using a VPN instead of a raw LAN

If clients connect over Tailscale, WireGuard, or similar, use the VPN-assigned address or hostname (e.g. Tailscale's MagicDNS name) in `MCP_ALLOWED_HOSTS` and in the client URL below — not the server's public or LAN IP. This keeps the endpoint unreachable from anyone outside the VPN, without needing TLS at all.

## Step 4 — Configure your AI client

Replace `192.168.1.50` with your server's LAN IP, VPN address, or hostname, and `your-secret-token` with the token from Step 2 (omit the `Authorization` header entirely if you decided to skip the token in Step 2).

### Claude Desktop

> **Note:** HTTP transport support in Claude Desktop is relatively new — make sure you're running the latest version.

```json
{
  "mcpServers": {
    "gramps-web": {
      "type": "http",
      "url": "http://192.168.1.50:3000/mcp/",
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
  gramps-web http://192.168.1.50:3000/mcp/ \
  -H "Authorization: Bearer your-secret-token"
```

Or add to `.mcp.json` in your project directory:

```json
{
  "mcpServers": {
    "gramps-web": {
      "type": "http",
      "url": "http://192.168.1.50:3000/mcp/",
      "headers": {
        "Authorization": "Bearer your-secret-token"
      }
    }
  }
}
```

> If the client fails to connect to `/mcp`, try `/mcp/` with a trailing slash — behavior can differ depending on how the request reaches the server.

## Keeping up to date

```bash
docker compose pull gramps-mcp && docker compose up -d gramps-mcp
```
