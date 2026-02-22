import { CodeBlock } from "@/components/code-block";

export default async function Configuration() {
  return (
    <>
      <h1>Configuration</h1>
      <p>
        PortZero can be configured with a <code>portzero.toml</code> file in
        your project root. This is useful for running multiple apps together.
      </p>

      <h2>Example configuration</h2>
      <CodeBlock
        lang="toml"
        filename="portzero.toml"
        code={`[proxy]
port = 1337
https = true

[apps.web]
command = "pnpm dev"
cwd = "./apps/web"
auto_restart = true

[apps.api]
command = "pnpm start"
cwd = "./apps/api"
subdomain = "api.myapp"`}
      />

      <h2>Proxy settings</h2>
      <table>
        <thead>
          <tr>
            <th>Key</th>
            <th>Type</th>
            <th>Default</th>
            <th>Description</th>
          </tr>
        </thead>
        <tbody>
          <tr>
            <td>
              <code>port</code>
            </td>
            <td>number</td>
            <td>1337</td>
            <td>The port the proxy listens on</td>
          </tr>
          <tr>
            <td>
              <code>https</code>
            </td>
            <td>boolean</td>
            <td>false</td>
            <td>Enable HTTPS with auto-generated certificates</td>
          </tr>
        </tbody>
      </table>

      <h2>App settings</h2>
      <p>
        Each app is defined under <code>[apps.&lt;name&gt;]</code>:
      </p>
      <table>
        <thead>
          <tr>
            <th>Key</th>
            <th>Type</th>
            <th>Default</th>
            <th>Description</th>
          </tr>
        </thead>
        <tbody>
          <tr>
            <td>
              <code>command</code>
            </td>
            <td>string</td>
            <td>required</td>
            <td>The command to run</td>
          </tr>
          <tr>
            <td>
              <code>cwd</code>
            </td>
            <td>string</td>
            <td>{'"."'}</td>
            <td>Working directory (relative to config file)</td>
          </tr>
          <tr>
            <td>
              <code>subdomain</code>
            </td>
            <td>string</td>
            <td>app name</td>
            <td>Custom subdomain for routing</td>
          </tr>
          <tr>
            <td>
              <code>auto_restart</code>
            </td>
            <td>boolean</td>
            <td>false</td>
            <td>Automatically restart on crash</td>
          </tr>
        </tbody>
      </table>

      <h2>Usage</h2>
      <CodeBlock
        lang="shellscript"
        code={`# Start all apps defined in portzero.toml
portzero up

# This will start:
# => http://web.localhost:1337
# => http://api.myapp.localhost:1337`}
      />

      <h2>Environment variables</h2>
      <p>
        PortZero automatically sets the <code>PORT</code> environment variable
        for each app. The port is deterministically assigned based on the app
        name, so it remains stable across restarts.
      </p>
    </>
  );
}
