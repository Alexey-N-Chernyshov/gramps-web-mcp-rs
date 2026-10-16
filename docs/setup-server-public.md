# Public server setup (HTTPS)

**Tested with:** gramps-web-mcp-rs v1.6.0, Gramps Web 26.10.0.

The server runs alongside Gramps Web and is reachable from the internet, behind the official Gramps Web Let's Encrypt setup (`nginx-proxy` + `acme-companion`; see [grampsweb.org/install_setup/lets_encrypt](https://www.grampsweb.org/install_setup/lets_encrypt/)).

**Prerequisites:** Gramps Web running on that setup, access to its `docker-compose.yml`, a domain name pointed at this server.

Use HTTPS on the standard port 443 — some MCP clients won't accept any other port (see the wiki's [Clients](https://github.com/Alexey-N-Chernyshov/gramps-web-mcp-rs/wiki/Clients) page). No domain of your own? A free dynamic-DNS hostname works the same way.

## 1. Choose an auth mode

Pick **one** — not both:

- **OAuth** — each person signs in with their own Gramps Web account; requires a stable HTTPS domain (`MCP_OAUTH_PUBLIC_URL` must be `https://`). No account to create here.
- **Static token** — one shared Gramps Web service account for everyone; doesn't need a domain itself (it also works without one, e.g. the [LAN/VPN setup](setup-server-lan.md)). Create that account now: role Editor for normal use, Member for read-only (pair with `GRAMPS_READONLY=true`); never Owner or Admin — see [roles.md](roles.md).

## 2. Choose a layout

- **[Own subdomain](#own-subdomain)** (recommended) — a new DNS record, its own certificate, completely isolated from Gramps Web.
- **[Shared host path](#shared-host-path)** — no new DNS record, served under `/mcp` on the same domain as Gramps Web.

## Own subdomain

```yaml
gramps-mcp:
  image: ghcr.io/alexey-n-chernyshov/gramps-web-mcp-rs:latest
  restart: always
  environment:
    GRAMPS_API_URL: http://grampsweb:5000
    MCP_TRANSPORT: http
    MCP_HTTP_PORT: 3000
    MCP_ALLOWED_HOSTS: gramps-mcp.example.com
    VIRTUAL_HOST: gramps-mcp.example.com
    LETSENCRYPT_HOST: gramps-mcp.example.com
    VIRTUAL_PORT: "3000"
  networks:
    - proxy-tier
    - default
```

No `ports:` — only `nginx-proxy`, on the `proxy-tier` network, should be able to reach this container.

Add the variables for whichever auth mode you picked in step 1:

### Auth: OAuth

```yaml
    MCP_OAUTH_ENABLED: "true"
    MCP_OAUTH_STORAGE_KEY: <openssl rand -base64 32>
    MCP_OAUTH_STORAGE_PATH: /data/oauth_store.json
    MCP_OAUTH_PUBLIC_URL: https://gramps-mcp.example.com
  volumes:
    - gramps_mcp_oauth:/data
```

And, at the top level of the compose file: `volumes: { gramps_mcp_oauth: }`. Before the first start, make that volume writable by the image's non-root user (uid 100, gid 101) — otherwise sessions are lost every time the container is recreated:

```bash
docker volume create gramps_mcp_oauth
docker run --rm -v gramps_mcp_oauth:/data alpine chown 100:101 /data
```

### Auth: static token

```yaml
    GRAMPS_USERNAME: gramps-mcp
    GRAMPS_PASSWORD: <password from step 1>
    MCP_AUTH_TOKEN: <openssl rand -hex 32>
```

**Client URL:** `https://gramps-mcp.example.com/mcp`

## Shared host path

Same `VIRTUAL_HOST` as `grampsweb`, with this server routed at `/mcp`:

```yaml
gramps-mcp:
  image: ghcr.io/alexey-n-chernyshov/gramps-web-mcp-rs:latest
  restart: always
  environment:
    GRAMPS_API_URL: http://grampsweb:5000
    MCP_TRANSPORT: http
    MCP_HTTP_PORT: 3000
    MCP_ALLOWED_HOSTS: gramps.example.com
    VIRTUAL_HOST: gramps.example.com   # same host as grampsweb
    VIRTUAL_PATH: /mcp
    VIRTUAL_PORT: "3000"
  networks:
    - proxy-tier
    - default
```

Add the variables for whichever auth mode you picked in step 1:

### Auth: OAuth

OAuth also needs its two discovery documents reachable at the host root (see [endpoints.md](endpoints.md)) — `VIRTUAL_PATH: /mcp` alone only routes `/mcp` itself there, so widen it:

```yaml
    MCP_OAUTH_ENABLED: "true"
    MCP_OAUTH_STORAGE_KEY: <openssl rand -base64 32>
    MCP_OAUTH_STORAGE_PATH: /data/oauth_store.json
    MCP_OAUTH_PUBLIC_URL: https://gramps.example.com/mcp
    VIRTUAL_PATH: ~^/(mcp(/.*)?|\.well-known/oauth-(protected-resource|authorization-server)/mcp)$
  volumes:
    - gramps_mcp_oauth:/data
```

Same top-level volume and one-time ownership fix as in [Own subdomain](#own-subdomain).

If you're hand-writing nginx config instead of relying on `nginx-proxy`'s templating, the equivalent is:

```nginx
location ~ ^/mcp(/.*)?$ {
    proxy_pass http://gramps-mcp:3000;
    proxy_http_version 1.1;
    proxy_set_header Host $host;
    proxy_set_header X-Forwarded-Proto $scheme;
}
location = /.well-known/oauth-protected-resource/mcp {
    proxy_pass http://gramps-mcp:3000;
    proxy_set_header Host $host;
}
location = /.well-known/oauth-authorization-server/mcp {
    proxy_pass http://gramps-mcp:3000;
    proxy_set_header Host $host;
}
```

> **UNVERIFIED — test on a real setup.** This `VIRTUAL_PATH` regex and its plain-nginx equivalent are derived from `nginx-proxy`'s documented behavior (it does not strip the matched path before proxying), not confirmed against a live `nginx-proxy` + `acme-companion` deployment. Please report back if you try it.

### Auth: static token

Works exactly as in [Own subdomain](#own-subdomain)'s static-token variant — add the same `GRAMPS_USERNAME`, `GRAMPS_PASSWORD`, and `MCP_AUTH_TOKEN`.

**Client URL:** `https://gramps.example.com/mcp`

## Connect your client

Use the URL and auth method from whichever layout you set up above. See the wiki's [Clients](https://github.com/Alexey-N-Chernyshov/gramps-web-mcp-rs/wiki/Clients) page for how to add an HTTP MCP server, with bearer-token or OAuth auth, to your specific client.

## Updating

```bash
docker compose pull gramps-mcp && docker compose up -d gramps-mcp
```
