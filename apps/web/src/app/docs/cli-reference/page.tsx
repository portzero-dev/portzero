import { CodeBlock } from "@/components/code-block";

export default async function CLIReference() {
  return (
    <>
      <h1>CLI Reference</h1>
      <p>
        Complete reference for the <code>portzero</code> command-line interface.
      </p>

      <h2>Running apps</h2>
      <p>
        PortZero can run commands as managed apps. When the first argument is a
        known executable (in <code>$PATH</code> or{" "}
        <code>./node_modules/.bin</code>), the app name is inferred from the
        current directory. Otherwise, the first argument is the app name and the
        rest is the command.
      </p>
      <table>
        <thead>
          <tr>
            <th>Command</th>
            <th>Description</th>
          </tr>
        </thead>
        <tbody>
          <tr>
            <td>
              <code>portzero &lt;command&gt;</code>
            </td>
            <td>Run command, name inferred from cwd</td>
          </tr>
          <tr>
            <td>
              <code>portzero &lt;name&gt; &lt;command&gt;</code>
            </td>
            <td>Run command with explicit name</td>
          </tr>
          <tr>
            <td>
              <code>portzero run &lt;name&gt; &lt;command...&gt;</code>
            </td>
            <td>Explicit form (supports <code>--no-restart</code>)</td>
          </tr>
          <tr>
            <td>
              <code>portzero up</code>
            </td>
            <td>
              Start all apps from <code>portzero.toml</code>
            </td>
          </tr>
          <tr>
            <td>
              <code>portzero down</code>
            </td>
            <td>Stop all running apps</td>
          </tr>
          <tr>
            <td>
              <code>portzero list</code>
            </td>
            <td>List active apps and URLs</td>
          </tr>
          <tr>
            <td>
              <code>portzero logs &lt;name&gt; [-n lines] [-f]</code>
            </td>
            <td>Tail logs for an app</td>
          </tr>
        </tbody>
      </table>

      <h3>
        <code>portzero run</code>
      </h3>
      <CodeBlock
        lang="shellscript"
        code={`# Explicit run with auto-restart disabled
portzero run my-app --no-restart pnpm dev

# Shorthand (equivalent, but auto-restart is on by default)
portzero my-app pnpm dev`}
      />

      <h3>
        <code>portzero logs</code>
      </h3>
      <CodeBlock
        lang="shellscript"
        code={`# Show last 50 lines
portzero logs my-app -n 50

# Follow logs in real time
portzero logs my-app -f`}
      />

      <h2>Daemon management</h2>
      <table>
        <thead>
          <tr>
            <th>Command</th>
            <th>Description</th>
          </tr>
        </thead>
        <tbody>
          <tr>
            <td>
              <code>portzero start</code>
            </td>
            <td>Start the proxy daemon (foreground)</td>
          </tr>
          <tr>
            <td>
              <code>portzero start -d</code>
            </td>
            <td>Start in background (daemonize)</td>
          </tr>
          <tr>
            <td>
              <code>portzero stop</code>
            </td>
            <td>Stop the daemon</td>
          </tr>
          <tr>
            <td>
              <code>portzero status</code>
            </td>
            <td>Show daemon status</td>
          </tr>
        </tbody>
      </table>

      <h2>Mocking</h2>
      <p>
        Mock rules let you return synthetic responses without hitting the
        upstream server. You can manage mocks from the{" "}
        <a href="/docs/desktop-app">desktop app</a> (recommended) or via the
        CLI.
      </p>
      <table>
        <thead>
          <tr>
            <th>Command</th>
            <th>Description</th>
          </tr>
        </thead>
        <tbody>
          <tr>
            <td>
              <code>portzero mock add &lt;app&gt; &lt;method&gt; &lt;path&gt;</code>
            </td>
            <td>Create a new mock rule</td>
          </tr>
          <tr>
            <td>
              <code>portzero mock list [--app &lt;name&gt;]</code>
            </td>
            <td>List all mock rules</td>
          </tr>
          <tr>
            <td>
              <code>portzero mock enable &lt;id&gt;</code>
            </td>
            <td>Enable a mock rule</td>
          </tr>
          <tr>
            <td>
              <code>portzero mock disable &lt;id&gt;</code>
            </td>
            <td>Disable a mock rule</td>
          </tr>
          <tr>
            <td>
              <code>portzero mock delete &lt;id&gt;</code>
            </td>
            <td>Delete a mock rule</td>
          </tr>
        </tbody>
      </table>

      <h3>
        <code>portzero mock add</code> options
      </h3>
      <table>
        <thead>
          <tr>
            <th>Flag</th>
            <th>Description</th>
            <th>Default</th>
          </tr>
        </thead>
        <tbody>
          <tr>
            <td>
              <code>--status &lt;code&gt;</code>
            </td>
            <td>Response status code</td>
            <td>200</td>
          </tr>
          <tr>
            <td>
              <code>--body &lt;string&gt;</code>
            </td>
            <td>Response body (inline)</td>
            <td>empty</td>
          </tr>
          <tr>
            <td>
              <code>--body-file &lt;path&gt;</code>
            </td>
            <td>Response body from file</td>
            <td>--</td>
          </tr>
          <tr>
            <td>
              <code>{"-H, --header <header>"}</code>
            </td>
            <td>
              Response header (<code>Key: Value</code>), repeatable
            </td>
            <td>--</td>
          </tr>
        </tbody>
      </table>

      <CodeBlock
        lang="shellscript"
        code={`# Mock a health check endpoint
portzero mock add api GET /health --status 200 \\
  --body '{"status":"ok"}' \\
  -H "Content-Type: application/json"

# Mock a 404 for a specific path
portzero mock add api GET /old-endpoint --status 404

# Mock with a body from file
portzero mock add api POST /webhook --body-file ./fixtures/webhook.json

# List mocks for a specific app
portzero mock list --app api

# Disable and delete
portzero mock disable abc123
portzero mock delete abc123`}
      />

      <h2>Network simulation</h2>
      <p>
        Test how your app behaves under degraded network conditions. Configure
        network simulation from the{" "}
        <a href="/docs/desktop-app">desktop app</a> (sliders per-app) or via
        the CLI.
      </p>
      <table>
        <thead>
          <tr>
            <th>Command</th>
            <th>Description</th>
          </tr>
        </thead>
        <tbody>
          <tr>
            <td>
              <code>portzero throttle set &lt;app&gt;</code>
            </td>
            <td>Set network simulation for an app</td>
          </tr>
          <tr>
            <td>
              <code>portzero throttle list</code>
            </td>
            <td>List active network simulation profiles</td>
          </tr>
          <tr>
            <td>
              <code>portzero throttle clear &lt;app&gt;</code>
            </td>
            <td>Clear network simulation for an app</td>
          </tr>
        </tbody>
      </table>

      <h3>
        <code>portzero throttle set</code> options
      </h3>
      <table>
        <thead>
          <tr>
            <th>Flag</th>
            <th>Description</th>
          </tr>
        </thead>
        <tbody>
          <tr>
            <td>
              <code>--latency &lt;ms&gt;</code>
            </td>
            <td>Fixed latency in milliseconds</td>
          </tr>
          <tr>
            <td>
              <code>--jitter &lt;ms&gt;</code>
            </td>
            <td>
              Random jitter +/- ms (requires <code>--latency</code>)
            </td>
          </tr>
          <tr>
            <td>
              <code>--drop &lt;0.0-1.0&gt;</code>
            </td>
            <td>Packet loss probability</td>
          </tr>
          <tr>
            <td>
              <code>--bandwidth &lt;bytes/s&gt;</code>
            </td>
            <td>Bandwidth limit in bytes per second</td>
          </tr>
          <tr>
            <td>
              <code>--path &lt;glob&gt;</code>
            </td>
            <td>Only apply to matching paths</td>
          </tr>
        </tbody>
      </table>

      <CodeBlock
        lang="shellscript"
        code={`# Add 200ms latency with 50ms jitter
portzero throttle set my-app --latency 200 --jitter 50

# Simulate 10% packet loss
portzero throttle set my-app --drop 0.1

# Limit bandwidth and add latency
portzero throttle set my-app --latency 100 --bandwidth 51200

# Only throttle a specific path
portzero throttle set my-app --latency 500 --path "/api/slow/*"

# List active profiles
portzero throttle list

# Clear simulation
portzero throttle clear my-app`}
      />

      <h2>Certificates</h2>
      <CodeBlock
        lang="shellscript"
        code={`portzero trust    # Install CA certificate into system trust store
portzero untrust  # Remove it`}
      />

      <h2>Public tunnels (Coming Soon)</h2>
      <blockquote>
        Public tunnel support is planned for a future release. When available,
        the <code>share</code> subcommand will expose local apps to the internet.
      </blockquote>
      <table>
        <thead>
          <tr>
            <th>Command</th>
            <th>Description</th>
          </tr>
        </thead>
        <tbody>
          <tr>
            <td>
              <code>portzero share start &lt;app&gt; [--subdomain] [--relay]</code>
            </td>
            <td>Expose an app via public tunnel</td>
          </tr>
          <tr>
            <td>
              <code>portzero share stop &lt;app&gt;</code>
            </td>
            <td>Stop sharing an app</td>
          </tr>
          <tr>
            <td>
              <code>portzero share list</code>
            </td>
            <td>List active tunnels</td>
          </tr>
        </tbody>
      </table>

      <h2>MCP server (Coming Soon)</h2>
      <p>MCP server for AI agent integration is planned for a future release.</p>
    </>
  );
}
