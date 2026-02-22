import { CodeBlock } from "@/components/code-block";

export default async function DocsHome() {
  return (
    <>
      <h1>PortZero Documentation</h1>
      <p>
        PortZero is a local development reverse proxy, process manager, and
        traffic inspector built in <strong>Rust</strong> using{" "}
        <strong>Cloudflare Pingora</strong> as the proxy engine. It assigns
        stable <code>{"<name>"}.localhost</code> URLs to your dev servers,
        captures all HTTP traffic for inspection, and provides request replay,
        mocking, interception, and network simulation -- all from a single
        binary.
      </p>

      <h2>Why PortZero?</h2>
      <ul>
        <li>
          <strong>Single binary</strong> -- No Node.js, no Docker, no runtime
          dependencies. Just one ~8MB Rust binary.
        </li>
        <li>
          <strong>Stable URLs</strong> -- Each app gets a predictable{" "}
          <code>name.localhost:1337</code> URL instead of random ports.
        </li>
        <li>
          <strong>Full observability</strong> -- Inspect every request and
          response, replay them, mock them, or simulate bad networks.
        </li>
        <li>
          <strong>AI-native (coming soon)</strong> -- MCP server will let AI
          coding agents inspect traffic and manage apps.
        </li>
        <li>
          <strong>Battle-tested proxy</strong> -- Powered by Cloudflare Pingora,
          the same engine behind Cloudflare{"'"}s global network.
        </li>
      </ul>

      <h2>Quick Example</h2>
      <CodeBlock
        lang="shellscript"
        code={`# Start a dev server with a stable URL
portzero next dev
# => http://my-app.localhost:1337

# Start multiple apps from a config file
portzero up
# => http://web.localhost:1337
# => http://api.localhost:1337`}
      />

      <h2>What{"'"}s in the docs</h2>
      <ul>
        <li>
          <strong>Getting Started</strong> -- Installation and first steps
        </li>
        <li>
          <strong>CLI Reference</strong> -- Every command explained
        </li>
        <li>
          <strong>Configuration</strong> -- The <code>portzero.toml</code> file
        </li>
        <li>
          <strong>Features</strong> -- Traffic inspector, replay, mocking,
          network simulation, and schema inference
        </li>
        <li>
          <strong>API Reference</strong> -- REST and WebSocket API
        </li>
        <li>
          <strong>MCP Server</strong> -- AI agent integration (coming soon)
        </li>
        <li>
          <strong>Desktop App</strong> -- Tauri v2 native dashboard
        </li>
        <li>
          <strong>Architecture</strong> -- How PortZero is built
        </li>
      </ul>
    </>
  );
}
