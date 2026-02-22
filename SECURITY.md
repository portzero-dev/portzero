# Security Policy

## Reporting a Vulnerability

If you discover a security vulnerability in PortZero, please report it responsibly.

**Do not open a public GitHub issue for security vulnerabilities.**

Instead, please email security concerns to the maintainers at the email address listed in the repository's GitHub profile, or use [GitHub's private vulnerability reporting](https://github.com/portzero/portzero/security/advisories/new).

### What to include

- A description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if any)

### Response timeline

- **Acknowledgment**: Within 48 hours
- **Assessment**: Within 1 week
- **Fix & disclosure**: We aim to release a fix within 30 days of confirmation, coordinating disclosure with the reporter

## Security Design

PortZero is a **local development tool** and its security model reflects that:

- The proxy binds to `127.0.0.1` only and is not network-accessible
- The API server and WebSocket are unauthenticated (acceptable for localhost-only dev tooling)
- Request bodies are capped at 1MB to prevent memory exhaustion
- The SQLite database is stored at `~/.portzero/` with `0600` permissions
- Auto-generated TLS certificates are scoped to `*.localhost` only
- The replay endpoint only sends requests to `127.0.0.1` (not an SSRF vector)
- The MCP server runs on stdio only (no network exposure)
- Tunnel connections use end-to-end encryption (QUIC/TLS) with JWT authentication
- Credentials are stored in `~/.portzero/credentials.json` with `0600` permissions

## Supported Versions

| Version | Supported |
|---------|-----------|
| 0.1.x   | Yes       |
