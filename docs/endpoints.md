# Endpoints

Every HTTP route the server exposes when `MCP_TRANSPORT=http`.

## Always present

| Path      | Method            | Auth                                                                                                                                   | Purpose                       |
| --------- | ----------------- | -------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------- |
| `/mcp`    | GET, POST, DELETE | Bearer token — static `MCP_AUTH_TOKEN`, or a per-user OAuth access token when `MCP_OAUTH_ENABLED=true`; none if neither is configured. | MCP Streamable HTTP endpoint. |
| `/health` | GET               | none                                                                                                                                   | Liveness check; returns `ok`. |

## OAuth mode only (`MCP_OAUTH_ENABLED=true`)

Where these mount depends on the path component of `MCP_OAUTH_PUBLIC_URL`. `/mcp` itself is always at `/mcp`, regardless of the issuer path.

### Bare-root issuer, e.g. `MCP_OAUTH_PUBLIC_URL=https://gramps-mcp.example.com`

| Path                                      | Method    | Auth                 | Purpose                                                                           |
| ----------------------------------------- | --------- | -------------------- | --------------------------------------------------------------------------------- |
| `/.well-known/oauth-protected-resource`   | GET       | none                 | RFC 9728 protected-resource metadata.                                             |
| `/.well-known/oauth-authorization-server` | GET       | none                 | RFC 8414 authorization-server metadata.                                           |
| `/register`                               | POST      | none                 | RFC 7591 dynamic client registration.                                             |
| `/authorize`                              | GET, POST | none                 | Login form: the user signs in with their own Gramps Web account.                  |
| `/token`                                  | POST      | none (PKCE-verified) | Exchanges an authorization code, or rotates a refresh token, for an access token. |

### Path-based issuer, e.g. `MCP_OAUTH_PUBLIC_URL=https://gramps.example.com/mcp` (shared host with Gramps Web itself)

| Path                                          | Method    | Auth                 | Purpose                                                                                                                                                                                                           |
| --------------------------------------------- | --------- | -------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `/.well-known/oauth-protected-resource/mcp`   | GET       | none                 | Protected-resource metadata, at the RFC 8414/9728 path-aware location: the `.well-known` segment sits at the host root, with the issuer's own path appended after it — never nested under `/mcp/.well-known/...`. |
| `/.well-known/oauth-authorization-server/mcp` | GET       | none                 | Authorization-server metadata, same path-aware placement.                                                                                                                                                         |
| `/mcp/register`                               | POST      | none                 | Dynamic client registration.                                                                                                                                                                                      |
| `/mcp/authorize`                              | GET, POST | none                 | Login form.                                                                                                                                                                                                       |
| `/mcp/token`                                  | POST      | none (PKCE-verified) | Token exchange / refresh.                                                                                                                                                                                         |

## Token lifetimes

- **Authorization code:** 60 seconds, single use.
- **Access token:** 15 minutes.
- **Refresh token:** rotates on every use (old one stops working immediately, OAuth 2.1-style); has no fixed expiry of its own. The underlying Gramps Web refresh token it's tied to doesn't expire on its own by default either (`JWT_REFRESH_TOKEN_EXPIRES=False` in `gramps-web-api`) — it dies only when Gramps Web rejects it: the user's account was deleted or disabled, or Gramps Web's `SECRET_KEY` changed. See [security.md](security.md#oauth-details) for what each of those actually does.
