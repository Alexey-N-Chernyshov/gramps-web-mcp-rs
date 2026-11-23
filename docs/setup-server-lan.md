# LAN / VPN setup (HTTP)

**Tested with:** gramps-web-mcp-rs v1.6.0, Gramps Web 26.10.0.

The server runs as its own service next to Gramps Web, reachable over plain HTTP from your LAN or VPN — no TLS, no public exposure.

**Prerequisites:** Gramps Web running via Docker Compose (26.10.0+), access to its `docker-compose.yml`, all clients on the same LAN or VPN (Tailscale, WireGuard, etc.).

## 1. Create a Gramps Web account for the MCP server

Role: Editor for normal use, Member for read-only (pair with `GRAMPS_READONLY=true`); never Owner or Admin — see [roles.md](roles.md).

## 2. Add the service

```yaml
gramps-mcp:
  image: ghcr.io/alexey-n-chernyshov/gramps-web-mcp-rs:latest
  restart: always
  environment:
    GRAMPS_API_URL: http://grampsweb:5000
    GRAMPS_USERNAME: gramps-mcp
    GRAMPS_PASSWORD: <password from step 1>
    MCP_TRANSPORT: http
    MCP_HTTP_PORT: 3000
    MCP_AUTH_TOKEN: <openssl rand -hex 32>   # optional on a fully trusted network, recommended otherwise
    MCP_ALLOWED_HOSTS: <host:port your clients actually use>   # exact LAN IP / VPN hostname, with port
  ports:
    - "3000:3000"
```

Full variable reference: [configuration.md](configuration.md).

## 3. Start it

```bash
docker compose up -d gramps-mcp
```

## 4. Verify

```bash
curl http://localhost:3000/health                     # from the server itself
curl http://<lan-ip-or-vpn-host>:3000/health           # from a client machine
```

Both must return `ok`. If the second fails while the first succeeds, check your firewall and that `MCP_ALLOWED_HOSTS` matches exactly what the client connects to.

## Reachability

This setup is reachable only from your LAN or VPN — not from the public internet. Some MCP clients connect from their provider's cloud rather than from your device; those clients can't reach it. See the wiki's [Clients](https://github.com/Alexey-N-Chernyshov/gramps-web-mcp-rs/wiki/Clients) page for which.

## Connect your client

- URL: `http://<lan-ip-or-vpn-host>:3000/mcp`
- Auth: Bearer token from step 2, if you set one.

See the wiki's [Clients](https://github.com/Alexey-N-Chernyshov/gramps-web-mcp-rs/wiki/Clients) page for how to add an HTTP MCP server to your specific client.

## Updating

```bash
docker compose pull gramps-mcp && docker compose up -d gramps-mcp
```
