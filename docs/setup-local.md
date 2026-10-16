# Local setup (stdio)

**Tested with:** gramps-web-mcp-rs v1.6.0, Gramps Web 26.10.0.

The MCP server runs as a child process of your AI client, talking over stdin/stdout — no network port opened, no auth token needed.

**Prerequisites:** a running [Gramps Web](https://www.grampsweb.org/) instance (26.10.0+), Docker on the client machine.

## 1. Create a Gramps Web account for the server

Role: Editor for normal use, Member for read-only (pair with `GRAMPS_READONLY=true`); never Owner or Admin — see [roles.md](roles.md).

## 2. Pull the image

```bash
docker pull ghcr.io/alexey-n-chernyshov/gramps-web-mcp-rs:latest
```

A client's own first launch attempt can otherwise time out while this downloads.

## 3. Work out how to reach Gramps Web from the container

If Gramps Web also runs in Docker on this machine, either:

- **Join its compose network** and address it by service name:
  ```bash
  docker run --rm -i --network <gramps-web-compose-project>_default \
    -e GRAMPS_API_URL=http://<grampsweb-service-name>:5000 ...
  ```
- **Or use the host gateway** instead:
  ```bash
  docker run --rm -i --add-host=host.docker.internal:host-gateway \
    -e GRAMPS_API_URL=http://host.docker.internal:5000 ...
  ```
  (`--add-host` is only needed on Linux; Docker Desktop already resolves `host.docker.internal`.)

If Gramps Web runs outside Docker, just use its normal URL.

## Connect your client

Give your client this launch command, with the values from steps 1 and 3:

```bash
docker run --rm -i \
  -e GRAMPS_API_URL=<your Gramps Web URL> \
  -e GRAMPS_USERNAME=<username from step 1> \
  -e GRAMPS_PASSWORD=<password from step 1> \
  [-e GRAMPS_READONLY=true] \
  ghcr.io/alexey-n-chernyshov/gramps-web-mcp-rs:latest
```

Full variable reference: [configuration.md](configuration.md). For how to add a stdio MCP server to your specific client, see the wiki's [Clients](https://github.com/Alexey-N-Chernyshov/gramps-web-mcp-rs/wiki/Clients) page.

## Updating

```bash
docker pull ghcr.io/alexey-n-chernyshov/gramps-web-mcp-rs:latest
```

Nothing to restart — your client launches a fresh container on its next connection.
