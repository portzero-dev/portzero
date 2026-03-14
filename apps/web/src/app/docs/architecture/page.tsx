import { Mermaid } from "@/components/mermaid";

export default function Architecture() {
  return (
    <>
      <h1>Architecture</h1>
      <p>
        PortZero is structured as a <strong>Cargo workspace</strong> with
        multiple crates, each responsible for a distinct concern.
      </p>

      <h2>Crate overview</h2>
      <table>
        <thead>
          <tr>
            <th>Crate</th>
            <th>Purpose</th>
          </tr>
        </thead>
        <tbody>
          <tr>
            <td>
              <code>portzero-core</code>
            </td>
            <td>
              Core library: proxy, router, recorder, process manager, mock
              engine, network sim, schema inference
            </td>
          </tr>
          <tr>
            <td>
              <code>portzero-cli</code>
            </td>
            <td>
              CLI binary (<code>portzero</code> command)
            </td>
          </tr>
          <tr>
            <td>
              <code>portzero-api</code>
            </td>
            <td>HTTP API server (axum) + WebSocket for real-time events</td>
          </tr>
          <tr>
            <td>
              <code>portzero-mcp</code>
            </td>
            <td>MCP server for AI agent integration</td>
          </tr>
          <tr>
            <td>
              <code>portzero-desktop</code>
            </td>
            <td>Tauri v2 desktop app</td>
          </tr>
        </tbody>
      </table>

      <h2>Crate dependency graph</h2>
      <Mermaid
        chart={`graph TD
  CLI["portzero-cli<br/><i>CLI binary</i>"]
  Desktop["portzero-desktop<br/><i>Tauri v2 app</i>"]
  API["portzero-api<br/><i>Axum HTTP + WS</i>"]
  MCP["portzero-mcp<br/><i>MCP server</i>"]
  Core["portzero-core<br/><i>Core library</i>"]

  CLI --> Core
  CLI --> API
  Desktop --> Core
  Desktop --> API
  MCP --> Core
  API --> Core

  style Core fill:#8b5cf6,stroke:#6d28d9,color:#fff
  style CLI fill:#27272a,stroke:#3f3f46,color:#e4e4e7
  style Desktop fill:#27272a,stroke:#3f3f46,color:#e4e4e7
  style API fill:#27272a,stroke:#3f3f46,color:#e4e4e7
  style MCP fill:#27272a,stroke:#3f3f46,color:#e4e4e7`}
        caption="Crate dependency graph"
      />

      <h2>Core components</h2>
      <p>
        The <code>portzero-core</code> crate is the heart of the system. It
        exports:
      </p>
      <ul>
        <li>
          <strong>Router</strong> -- Maps subdomains to local ports
        </li>
        <li>
          <strong>Recorder</strong> -- Captures request/response pairs to SQLite
        </li>
        <li>
          <strong>ProcessManager</strong> -- Spawns, monitors, and restarts
          child processes
        </li>
        <li>
          <strong>MockEngine</strong> -- Matches requests against mock rules and
          returns synthetic responses
        </li>
        <li>
          <strong>NetworkSim</strong> -- Applies latency, loss, and bandwidth
          limits per-app
        </li>
        <li>
          <strong>SchemaInference</strong> -- Builds OpenAPI schemas from
          observed traffic
        </li>
        <li>
          <strong>TunnelManager</strong> -- Manages public tunnels via LocalUp
        </li>
        <li>
          <strong>Store</strong> -- SQLite persistence layer (WAL mode, r2d2
          pool)
        </li>
        <li>
          <strong>WsHub</strong> -- WebSocket event broadcast hub
        </li>
      </ul>

      <h2>Proxy engine</h2>
      <p>
        PortZero uses{" "}
        <a
          href="https://github.com/cloudflare/pingora"
          target="_blank"
          rel="noopener noreferrer"
        >
          Cloudflare Pingora
        </a>{" "}
        as its proxy engine. Pingora is a battle-tested, multi-threaded async
        proxy framework used in production at Cloudflare. It provides:
      </p>
      <ul>
        <li>HTTP/1.1 and HTTP/2 support</li>
        <li>Native WebSocket upgrade handling</li>
        <li>Connection pooling and keep-alive</li>
        <li>Graceful shutdown and reload</li>
      </ul>

      <h2>Data flow</h2>
      <Mermaid
        chart={`flowchart TD
  Client["Browser / curl / agent"]
  Proxy["Pingora ProxyHttp<br/><i>*.localhost:1337</i>"]
  Router["Router<br/><i>subdomain → port</i>"]
  Recorder["Recorder<br/><i>capture traffic</i>"]
  SQLite[("SQLite<br/><i>WAL mode</i>")]
  Intercept["Intercept Engine<br/><i>breakpoints</i>"]
  Mock["Mock Engine<br/><i>synthetic responses</i>"]
  NetSim["Network Sim<br/><i>latency / loss / bw</i>"]
  PM["Process Manager<br/><i>spawn & restart</i>"]
  Upstream["Upstream App<br/><i>localhost:port</i>"]
  WsHub["WsHub<br/><i>real-time events</i>"]
  API["API Server<br/><i>REST + WebSocket</i>"]
  Dashboard["Desktop App / Web UI"]

  Client -->|HTTP request| Proxy
  Proxy --> Router
  Proxy --> Recorder
  Recorder --> SQLite
  Router --> Intercept
  Router --> Mock
  Router --> NetSim
  NetSim --> Upstream
  PM -->|manages| Upstream
  Recorder --> WsHub
  WsHub --> API
  API --> Dashboard

  style Client fill:#27272a,stroke:#3f3f46,color:#e4e4e7
  style Proxy fill:#8b5cf6,stroke:#6d28d9,color:#fff
  style SQLite fill:#27272a,stroke:#8b5cf6,color:#c4b5fd
  style Upstream fill:#27272a,stroke:#3f3f46,color:#e4e4e7
  style Dashboard fill:#27272a,stroke:#3f3f46,color:#e4e4e7
  style Router fill:#18181b,stroke:#3f3f46,color:#e4e4e7
  style Recorder fill:#18181b,stroke:#3f3f46,color:#e4e4e7
  style Intercept fill:#18181b,stroke:#3f3f46,color:#e4e4e7
  style Mock fill:#18181b,stroke:#3f3f46,color:#e4e4e7
  style NetSim fill:#18181b,stroke:#3f3f46,color:#e4e4e7
  style PM fill:#18181b,stroke:#3f3f46,color:#e4e4e7
  style WsHub fill:#18181b,stroke:#3f3f46,color:#e4e4e7
  style API fill:#18181b,stroke:#3f3f46,color:#e4e4e7`}
        caption="Request lifecycle through the proxy"
      />

      <h2>State management</h2>
      <p>
        All persistent state is stored in <strong>SQLite</strong> using WAL
        (Write-Ahead Logging) mode for concurrent read performance. The database
        is managed via <code>rusqlite</code> with an <code>r2d2</code>{" "}
        connection pool.
      </p>
      <p>Stored data includes:</p>
      <ul>
        <li>Captured request/response records</li>
        <li>Mock rules</li>
        <li>App registrations and status</li>
        <li>Process logs</li>
        <li>Inferred API schemas</li>
      </ul>

      <h2>Learn more</h2>
      <p>
        For the full 1300+ line architecture document, see{" "}
        <a
          href="https://github.com/portzero-dev/portzero/blob/main/ARCHITECTURE.md"
          target="_blank"
          rel="noopener noreferrer"
        >
          ARCHITECTURE.md
        </a>{" "}
        in the repository.
      </p>
    </>
  );
}
