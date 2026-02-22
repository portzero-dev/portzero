import Image from "next/image";
import { CodeBlock } from "@/components/code-block";

export default async function DesktopApp() {
  return (
    <>
      <h1>Desktop App</h1>
      <p>
        PortZero includes a native desktop application built with{" "}
        <strong>Tauri v2</strong> and <strong>React 19</strong>. It provides a
        graphical interface for all PortZero features and is the recommended way
        to manage mocks, network simulation, and traffic inspection.
      </p>

      <h2>Dashboard</h2>
      <p>
        The main dashboard shows all running apps at a glance with live stats
        including port, uptime, CPU, and memory usage. A live traffic feed
        streams requests as they flow through the proxy.
      </p>
      <Image
        src="/screenshots/dashboard.png"
        alt="PortZero Dashboard -- overview of running apps with live traffic feed"
        width={1456}
        height={816}
        className="my-4 rounded-lg border border-zinc-800"
      />

      <h2>Traffic Inspector</h2>
      <p>
        Browse, filter, and search every HTTP request flowing through the proxy.
        Click any request to see full headers and body with syntax highlighting.
        Replay requests with one click, and compare responses side-by-side with
        the built-in diff viewer.
      </p>
      <Image
        src="/screenshots/traffic.png"
        alt="PortZero Traffic Inspector -- full request/response capture with filtering"
        width={1456}
        height={816}
        className="my-4 rounded-lg border border-zinc-800"
      />

      <h2>Response Mocking</h2>
      <p>
        Create and manage mock responses directly from the desktop app. Set the
        HTTP method, path pattern, status code, response headers, and body. Mock
        rules can be enabled, disabled, or deleted at any time.
      </p>
      <Image
        src="/screenshots/mocks.png"
        alt="PortZero Mocks -- create mock responses for API endpoints"
        width={1456}
        height={816}
        className="my-4 rounded-lg border border-zinc-800"
      />

      <h2>Settings</h2>
      <p>
        Manage the proxy daemon, install the CLI tool, and trust HTTPS
        certificates -- all from one place. The settings page also shows the
        daemon status and log output.
      </p>
      <Image
        src="/screenshots/settings.png"
        alt="PortZero Settings -- daemon management, CLI install, and HTTPS certificates"
        width={1456}
        height={816}
        className="my-4 rounded-lg border border-zinc-800"
      />

      <h2>All features</h2>
      <ul>
        <li>
          <strong>Overview dashboard</strong> -- See all running apps at a
          glance with live stats
        </li>
        <li>
          <strong>Traffic inspector</strong> -- Browse, filter, and search
          captured requests
        </li>
        <li>
          <strong>Request detail</strong> -- Full headers and body with
          syntax highlighting
        </li>
        <li>
          <strong>Request replay</strong> -- One-click re-send with diff view
        </li>
        <li>
          <strong>Mock rules</strong> -- Create, edit, and toggle mock
          responses
        </li>
        <li>
          <strong>Network simulation</strong> -- Sliders for latency, jitter,
          loss, and bandwidth per-app
        </li>
        <li>
          <strong>Live logs</strong> -- Per-app log streaming in the dashboard
        </li>
        <li>
          <strong>API schema</strong> -- View inferred OpenAPI schemas
        </li>
        <li>
          <strong>System tray</strong> -- Quick access from the menu bar
        </li>
      </ul>

      <h2>Building from source</h2>
      <CodeBlock
        lang="shellscript"
        code={`# Prerequisites
# - Rust 1.75+
# - Node.js 18+ and pnpm

# Install frontend dependencies
cd apps/desktop
pnpm install

# Development mode
pnpm tauri dev

# Production build
pnpm tauri build`}
      />

      <h2>Web dashboard fallback</h2>
      <p>
        If you don{"'"}t want to install the desktop app, PortZero also serves
        the same dashboard as an embedded web app at:
      </p>
      <CodeBlock lang="text" code="http://_portzero.localhost:1337" />
      <p>
        This works in any browser and provides the same functionality as the
        desktop app, except for system tray integration and deep links.
      </p>

      <h2>Deep links</h2>
      <p>
        The desktop app registers the <code>portzero://</code> URL scheme. This
        allows other tools to open the dashboard directly to a specific view:
      </p>
      <CodeBlock
        lang="text"
        code={`portzero://apps/my-app       # Open app detail
portzero://traffic/abc123   # Open request detail`}
      />
    </>
  );
}
