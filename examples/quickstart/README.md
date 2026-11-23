# Quickstart

A disposable Gramps Web instance for trying this MCP server locally.

```bash
curl -O https://raw.githubusercontent.com/Alexey-N-Chernyshov/gramps-web-mcp-rs/main/examples/quickstart/docker-compose.yml
docker compose up -d
```

Open <http://localhost:5000>, log in with `gramps-mcp` / `gramps-mcp`.

Pull the image first (a client's own first launch attempt can otherwise time out), then point your MCP client at this stdio command:

```bash
docker pull ghcr.io/alexey-n-chernyshov/gramps-web-mcp-rs:latest
docker run --rm -i --network gramps-quickstart_default \
  -e GRAMPS_API_URL=http://grampsweb:5000 \
  -e GRAMPS_USERNAME=gramps-mcp -e GRAMPS_PASSWORD=gramps-mcp \
  ghcr.io/alexey-n-chernyshov/gramps-web-mcp-rs:latest
```

Cleanup: `docker compose down` (keeps data) or `docker compose down -v` (deletes it).

Full walkthrough: wiki [Quickstart](https://github.com/Alexey-N-Chernyshov/gramps-web-mcp-rs/wiki/Quickstart).
