import { CodeBlock } from "@/components/code-block";

export default async function GettingStarted() {
  return (
    <>
      <h1>Getting Started</h1>
      <p>
        Get PortZero installed and running your first app in under a minute.
      </p>

      <h2>Prerequisites</h2>
      <ul>
        <li>macOS (Apple Silicon or Intel) or Linux (x86_64 or arm64)</li>
      </ul>

      <h2>Installation</h2>
      <h3>Homebrew (recommended)</h3>
      <CodeBlock
        lang="shellscript"
        code="brew install portzero-dev/tap/portzero"
      />

      <h3>Quick install script</h3>
      <p>
        Install the latest release with a single command:
      </p>
      <CodeBlock
        lang="shellscript"
        code="curl -fsSL https://goport0.dev/install.sh | bash"
      />
      <p>
        This downloads the prebuilt binary for your platform and installs it
        to <code>/usr/local/bin</code>. You can override the install directory:
      </p>
      <CodeBlock
        lang="shellscript"
        code={`# Install to ~/.local/bin instead
PORTZERO_INSTALL_DIR=~/.local/bin curl -fsSL https://goport0.dev/install.sh | bash

# Install a specific version
PORTZERO_VERSION=0.1.0 curl -fsSL https://goport0.dev/install.sh | bash`}
      />

      <h3>From GitHub releases</h3>
      <p>
        Download the binary for your platform from the{" "}
        <a
          href="https://github.com/portzero-dev/portzero/releases"
          target="_blank"
          rel="noopener noreferrer"
        >
          GitHub Releases
        </a>{" "}
        page:
      </p>
      <table>
        <thead>
          <tr>
            <th>Platform</th>
            <th>Asset</th>
          </tr>
        </thead>
        <tbody>
          <tr>
            <td>macOS (Apple Silicon)</td>
            <td>
              <code>portzero-darwin-aarch64.tar.gz</code>
            </td>
          </tr>
          <tr>
            <td>macOS (Intel)</td>
            <td>
              <code>portzero-darwin-x86_64.tar.gz</code>
            </td>
          </tr>
          <tr>
            <td>Linux (x86_64)</td>
            <td>
              <code>portzero-linux-x86_64.tar.gz</code>
            </td>
          </tr>
          <tr>
            <td>Linux (arm64)</td>
            <td>
              <code>portzero-linux-aarch64.tar.gz</code>
            </td>
          </tr>
        </tbody>
      </table>
      <CodeBlock
        lang="shellscript"
        code={`# Example: manual install on macOS Apple Silicon
curl -fsSL https://github.com/portzero-dev/portzero/releases/latest/download/portzero-darwin-aarch64.tar.gz | tar xz
sudo mv portzero /usr/local/bin/`}
      />

      <h3>From source</h3>
      <p>
        Requires Rust 1.77+ and a C compiler:
      </p>
      <CodeBlock
        lang="shellscript"
        code={`git clone https://github.com/portzero-dev/portzero.git
cd portzero
cargo install --path crates/portzero-cli`}
      />

      <h2>Your first app</h2>
      <p>
        Navigate to any project directory and prefix your dev command with{" "}
        <code>portzero</code>:
      </p>
      <CodeBlock
        lang="shellscript"
        code={`cd my-project
portzero next dev
# => http://my-project.localhost:1337`}
      />
      <p>
        The name is automatically inferred from the current directory name.
        PortZero will:
      </p>
      <ul>
        <li>
          Start your command (<code>next dev</code>) with a deterministic{" "}
          <code>PORT</code> env variable
        </li>
        <li>
          Route <code>my-project.localhost:1337</code> to that port
        </li>
        <li>Capture all HTTP traffic for inspection</li>
        <li>Monitor the process and auto-restart on crash</li>
      </ul>

      <h2>Explicit naming</h2>
      <p>You can also provide a name explicitly:</p>
      <CodeBlock
        lang="shellscript"
        code={`portzero my-api cargo run
# => http://my-api.localhost:1337`}
      />

      <h2>HTTPS support</h2>
      <p>Trust the auto-generated local CA certificate for HTTPS:</p>
      <CodeBlock
        lang="shellscript"
        code={`portzero trust    # Installs CA into system trust store
portzero untrust  # Removes it`}
      />

      <h2>Next steps</h2>
      <ul>
        <li>
          Learn the full <a href="/docs/cli-reference">CLI Reference</a>
        </li>
        <li>
          Set up multi-app config with{" "}
          <a href="/docs/configuration">portzero.toml</a>
        </li>
        <li>
          Explore the{" "}
          <a href="/docs/features">Traffic Inspector and other features</a>
        </li>
      </ul>
    </>
  );
}
