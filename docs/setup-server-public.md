# Server-side setup (HTTP)

The MCP server runs alongside Gramps Web and reachable from the internet, via reverse proxy. The AI client connects over the network. No Docker required on the client machine.

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

The endpoint **must** be served over HTTPS if it's reachable from the public internet — the bearer token alone is not safe over plain HTTP. The simplest way to get HTTPS without writing any nginx config by hand is to reuse the same reverse proxy that secures Gramps Web itself.

If you're running Gramps Web's official Let's Encrypt setup (`nginx-proxy` + `acme-companion`, see [grampsweb.org/install_setup/lets_encrypt](https://www.grampsweb.org/install_setup/lets_encrypt/)), add `gramps-mcp` to the _same_ `docker-compose.yml`, on the _same_ `VIRTUAL_HOST` as `grampsweb` — no new subdomain or certificate needed:

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
    VIRTUAL_HOST: gramps.example.com # same host as grampsweb
    VIRTUAL_PATH: /mcp/
    VIRTUAL_PORT: "3000"
    MCP_ALLOWED_HOSTS: gramps.example.com # required — see note below
  networks:
    - proxy-tier
    - default
```

> **Why `MCP_ALLOWED_HOSTS` is required here:** the server's default DNS-rebinding protection only allows `localhost`/`127.0.0.1`. Behind `nginx-proxy`, the backend sees the original `Host` header from the client (e.g. `gramps.example.com`), not `localhost` — so without `MCP_ALLOWED_HOSTS` set, every request through the proxy gets rejected by the MCP server itself, even though nginx is routing correctly. Set it to the exact host (and port, if non-standard) clients will connect to.

Don't add a `ports:` entry — only `nginx-proxy` on the `proxy-tier` network should be able to reach port 3000.

Start the service:

```bash
docker compose up -d gramps-mcp
```

**No domain at all?** Let's Encrypt can't issue a certificate for a bare IP. Either keep the MCP server LAN/VPN-only over plain HTTP, or point a free dynamic-DNS hostname at your IP and follow the steps above as-is.

## Step 4 — Configure your AI client

Replace `gramps.example.com` with your actual domain, and `your-secret-token` with the token from Step 2. The URL always uses `https://`.

### Claude Code

Via CLI:

```bash
claude mcp add --transport http --scope project \
  gramps-web https://gramps.example.com/mcp/ \
  -H "Authorization: Bearer your-secret-token"
```

Or add to `.mcp.json` in your project directory:

```json
{
  "mcpServers": {
    "gramps-web": {
      "type": "http",
      "url": "https://gramps.example.com/mcp/",
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
