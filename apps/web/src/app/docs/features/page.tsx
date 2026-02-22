import Image from "next/image";
import { CodeBlock } from "@/components/code-block";

export default async function Features() {
  return (
    <>
      <h1>Features</h1>
      <p>
        PortZero includes a comprehensive set of dev tools beyond basic reverse
        proxying. Most features are accessible both through the{" "}
        <a href="/docs/desktop-app">desktop app</a> and the{" "}
        <a href="/docs/cli-reference">CLI</a>.
      </p>

      <h2>Traffic Inspector</h2>
      <p>
        Every HTTP request and response passing through PortZero is captured and
        stored in SQLite (WAL mode for performance). You can inspect traffic
        through the desktop app or the REST API.
      </p>
      <Image
        src="/screenshots/traffic.png"
        alt="Traffic Inspector -- browse, filter, and search captured requests"
        width={1456}
        height={816}
        className="my-4 rounded-lg border border-zinc-800"
      />
      <ul>
        <li>Full request/response headers and bodies</li>
        <li>Filter by app, status code, HTTP method, path, or full-text search</li>
        <li>Persistent storage across daemon restarts</li>
        <li>Real-time streaming via WebSocket</li>
      </ul>

      <h2>Request Replay</h2>
      <p>
        Re-send any captured request with a single click in the desktop app or
        via the API. You can optionally override headers, body, or URL before
        replaying.
      </p>
      <CodeBlock lang="http" code="POST /api/requests/:id/replay" />
      <p>
        The replayed response is captured alongside the original, and you can use
        the diff viewer to compare them side-by-side.
      </p>

      <h2>Response Mocking</h2>
      <p>
        Create synthetic responses for specific routes without hitting the
        upstream server. Useful for testing error states, edge cases, or when the
        backend is unavailable.
      </p>
      <p>
        The easiest way to manage mocks is through the{" "}
        <strong>desktop app</strong>, where you can create, edit, enable/disable,
        and delete mock rules with a visual editor:
      </p>
      <Image
        src="/screenshots/mocks.png"
        alt="Response Mocking -- create and manage mock rules in the desktop app"
        width={1456}
        height={816}
        className="my-4 rounded-lg border border-zinc-800"
      />
      <p>
        You can also manage mocks via the CLI:
      </p>
      <CodeBlock
        lang="shellscript"
        code={`# Create a mock rule
portzero mock add api GET /health --status 200 \\
  --body '{"status":"ok"}' \\
  -H "Content-Type: application/json"

# List all mocks
portzero mock list --app api

# Enable/disable/delete by ID
portzero mock enable abc123
portzero mock disable abc123
portzero mock delete abc123`}
      />

      <h2>Request Interception</h2>
      <p>
        Set breakpoints on specific routes to pause, inspect, edit, and
        forward or drop live requests before they reach the upstream server.
      </p>

      <h2>Network Simulation</h2>
      <p>
        Test how your app behaves under degraded network conditions. The desktop
        app provides intuitive sliders for per-app configuration:
      </p>
      <ul>
        <li>
          <strong>Latency injection</strong> -- Add fixed or random delay to
          responses
        </li>
        <li>
          <strong>Jitter</strong> -- Add randomness to latency for realistic
          simulation
        </li>
        <li>
          <strong>Packet loss</strong> -- Randomly drop a percentage of
          requests
        </li>
        <li>
          <strong>Bandwidth throttling</strong> -- Limit response throughput
        </li>
        <li>
          <strong>Path filtering</strong> -- Only apply simulation to matching
          routes
        </li>
      </ul>
      <p>
        Via the CLI:
      </p>
      <CodeBlock
        lang="shellscript"
        code={`# Add 200ms latency with 50ms jitter
portzero throttle set my-app --latency 200 --jitter 50

# Simulate 10% packet loss
portzero throttle set my-app --drop 0.1

# Clear simulation
portzero throttle clear my-app`}
      />

      <h2>API Schema Inference</h2>
      <p>
        PortZero passively observes traffic and infers an OpenAPI schema from
        actual requests and responses. No manual specification needed -- the
        schema builds up automatically as you use your API.
      </p>

      <h2>Public Tunnels (Coming Soon)</h2>
      <blockquote>
        This feature is not yet available. It is planned for a future release.
      </blockquote>
      <p>
        Expose local apps to the internet with a single command. Will support
        QUIC, WebSocket, and HTTP/2 transport.
      </p>
    </>
  );
}
