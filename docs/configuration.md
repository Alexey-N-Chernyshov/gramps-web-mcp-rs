# Configuration

All configuration is via environment variables.

| Variable                 | Default              | Required when                           | Description                                                                                                                                                                                                                              |
| ------------------------ | -------------------- | --------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `GRAMPS_API_URL`         | —                    | always                                  | Base URL of your Gramps Web instance, e.g. `https://gramps.example.com` or `http://localhost:5000`. Must include the scheme.                                                                                                             |
| `GRAMPS_USERNAME`        | —                    | always, unless `MCP_OAUTH_ENABLED=true` | Gramps Web username for the shared service account.                                                                                                                                                                                      |
| `GRAMPS_PASSWORD`        | —                    | always, unless `MCP_OAUTH_ENABLED=true` | Gramps Web password for the shared service account.                                                                                                                                                                                      |
| `GRAMPS_READONLY`        | `false`              | —                                       | Set to `true` to hide write tools from the AI client. This is a convenience flag, not a security boundary — the account's own Gramps Web role still applies regardless of this setting. See [roles.md](roles.md) and `security.md`.      |
| `MCP_TRANSPORT`          | `stdio`              | —                                       | `stdio` or `http`.                                                                                                                                                                                                                       |
| `MCP_HTTP_HOST`          | `0.0.0.0`            | —                                       | Bind host. Only used when `MCP_TRANSPORT=http`.                                                                                                                                                                                          |
| `MCP_HTTP_PORT`          | `3000`               | —                                       | Bind port. Only used when `MCP_TRANSPORT=http`.                                                                                                                                                                                          |
| `MCP_AUTH_TOKEN`         | —                    | —                                       | Static bearer token protecting `/mcp`. An empty string is treated as unset. Ignored when `MCP_OAUTH_ENABLED=true` — if unset in that mode, OAuth is the only way in; if unset and OAuth is off too, `/mcp` has no authentication at all. |
| `MCP_ALLOWED_HOSTS`      | —                    | —                                       | Comma-separated allowed `Host` header values (DNS-rebinding protection) for HTTP transport. If unset, only `localhost`/`127.0.0.1`/`::1` are allowed. Example: `your-domain.com:8888,localhost,127.0.0.1`.                               |
| `MCP_KEEP_ALIVE`         | `300`                | —                                       | HTTP session keep-alive timeout, in seconds. `0` disables session expiration.                                                                                                                                                            |
| `MCP_MAX_MEDIA_BYTES`    | `52428800` (50 MiB)  | —                                       | Maximum size accepted for a media file read or upload.                                                                                                                                                                                   |
| `MCP_OAUTH_ENABLED`      | `false`              | —                                       | Turns on per-user OAuth 2.1 on the HTTP transport instead of the single static identity from `GRAMPS_USERNAME`/`GRAMPS_PASSWORD`.                                                                                                        |
| `MCP_OAUTH_STORAGE_KEY`  | —                    | when `MCP_OAUTH_ENABLED=true`           | Base64 of exactly 32 bytes, encrypting the on-disk OAuth store. Generate with `openssl rand -base64 32`.                                                                                                                                 |
| `MCP_OAUTH_STORAGE_PATH` | `./oauth_store.json` | —                                       | Where encrypted OAuth state (registered clients, sessions) is persisted. Must be on a volume that survives a restart.                                                                                                                    |
| `MCP_OAUTH_PUBLIC_URL`   | —                    | when `MCP_OAUTH_ENABLED=true`           | This server's externally-visible base URL. Must be `https://`. Its path decides where the OAuth routes and discovery documents are mounted — see [endpoints.md](endpoints.md).                                                           |

## Examples

Local, stdio:

```bash
GRAMPS_API_URL=https://gramps.example.com
GRAMPS_USERNAME=gramps-mcp
GRAMPS_PASSWORD=your-password
```

HTTP with a static token:

```bash
GRAMPS_API_URL=https://gramps.example.com
GRAMPS_USERNAME=gramps-mcp
GRAMPS_PASSWORD=your-password
MCP_TRANSPORT=http
MCP_AUTH_TOKEN=your-secret-token
MCP_ALLOWED_HOSTS=your-domain.com:8443
```

HTTP with OAuth (no shared `GRAMPS_USERNAME`/`GRAMPS_PASSWORD` — each user signs in with their own):

```bash
GRAMPS_API_URL=https://gramps.example.com
MCP_TRANSPORT=http
MCP_OAUTH_ENABLED=true
MCP_OAUTH_STORAGE_KEY=<output of: openssl rand -base64 32>
MCP_OAUTH_PUBLIC_URL=https://gramps-mcp.example.com
MCP_ALLOWED_HOSTS=gramps-mcp.example.com
```

See the setup guides for how these are used in each deployment mode:
[local](setup-local.md), [LAN/VPN](setup-server-lan.md), [public server](setup-server-public.md).
