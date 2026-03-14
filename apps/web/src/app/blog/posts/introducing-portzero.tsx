import { CodeBlock } from "@/components/code-block";

export default async function IntroducingPortZero() {
  return (
    <div className="docs-content">
      <h2>The Problem</h2>
      <p>
        If you work on more than one microservice at a time, you know the drill.
        You spin up a frontend on <code>:3000</code>, an API on{" "}
        <code>:8080</code>, maybe a worker on <code>:9090</code>. You juggle
        ports, forget which service runs where, and when something breaks you
        have no idea what request caused it.
      </p>
      <p>
        Your browser history is full of <code>localhost:3000</code>,{" "}
        <code>localhost:3001</code>, <code>localhost:8080</code>. You mix them up
        between projects. And if you need someone else to see your local work?
        Time to set up a tunnel.
      </p>
      <p>
        I hit this problem daily. I tried the existing tools. Each solved part of
        it, but none solved all of it. So I built PortZero.
      </p>

      <h2>What I Was Using Before</h2>

      <h3>Nginx / Caddy / Traefik</h3>
      <p>
        Great reverse proxies — for production. For local dev, they&apos;re
        overkill. You need config files, you need to manage them separately from
        your services, and they don&apos;t know anything about your dev
        processes. No traffic inspection, no process management, no hot-reload
        awareness. They&apos;re designed for deployed infrastructure, not for{" "}
        <code>pnpm dev</code>.
      </p>

      <h3>Ngrok</h3>
      <p>
        Ngrok is excellent at what it does: exposing local ports to the internet.
        But it&apos;s a tunnel, not a local dev environment tool. You still need
        to manage ports yourself, it doesn&apos;t give you stable local URLs
        across projects, and there&apos;s no built-in traffic inspection,
        mocking, or process management. It solves the &quot;show this to a
        teammate&quot; problem, not the &quot;I have 5 services running
        locally&quot; problem.
      </p>

      <h3>Portless (Vercel)</h3>
      <p>
        <a
          href="https://github.com/nicepkg/portless"
          target="_blank"
          rel="noopener noreferrer"
        >
          Portless
        </a>{" "}
        tackles the same core problem: giving your local dev servers human-readable
        URLs instead of random port numbers. It&apos;s a CLI tool from the Vercel
        ecosystem that maps <code>.local</code> domains to your services.
      </p>
      <p>
        Portless validated the problem — port juggling is real and developers
        hate it. But it&apos;s CLI-only with no visual dashboard, no traffic
        inspection, no request replay or mocking, and no process management. I
        wanted something that went further: not just stable URLs, but a full
        local dev companion that lets you see, debug, and simulate everything
        flowing through your services.
      </p>

      <h2>What PortZero Does Differently</h2>
      <p>
        PortZero is a single Rust binary that combines{" "}
        <a href="/docs/features">five things</a> that usually require separate
        tools:
      </p>
      <ol>
        <li>
          <strong>Reverse proxy</strong> — route{" "}
          <code>&lt;app&gt;.localhost</code> to your local ports, powered by
          Cloudflare Pingora
        </li>
        <li>
          <strong>Process manager</strong> — spawn, monitor, and auto-restart
          your dev services with deterministic port assignment
        </li>
        <li>
          <strong>Traffic inspector</strong> — capture every HTTP request and
          response with full headers and bodies, persisted to SQLite
        </li>
        <li>
          <strong>Request replay &amp; mocking</strong> — re-send captured
          requests with overrides, or create mock responses without hitting
          upstream
        </li>
        <li>
          <strong>Network simulation</strong> — inject latency, packet loss, and
          bandwidth throttling per-app to test degraded conditions
        </li>
      </ol>

      <p>Here&apos;s what it looks like in practice:</p>

      <CodeBlock
        lang="shellscript"
        code={`# Start your frontend — it gets http://web.localhost:1337
$ portzero web pnpm dev

# Start your API — it gets http://api.localhost:1337
$ portzero api cargo run

# See everything running
$ portzero list
  web   Running  http://web.localhost:1337  (pid 12345)
  api   Running  http://api.localhost:1337  (pid 12346)

# Tail logs from your API
$ portzero logs api -f`}
      />

      <p>No config files. No port numbers to remember. Just names.</p>

      <h2>Why Rust?</h2>
      <p>
        A dev proxy sits in the hot path of every HTTP request you make during
        development. It needs to be fast enough that you forget it&apos;s there.
        Rust gives us that — sub-millisecond proxy overhead — plus a single
        static binary with no runtime dependencies.
      </p>
      <p>
        We built on top of{" "}
        <a
          href="https://github.com/cloudflare/pingora"
          target="_blank"
          rel="noopener noreferrer"
        >
          Cloudflare Pingora
        </a>
        , the same proxy framework that handles a significant chunk of
        internet traffic at Cloudflare. It gives us HTTP/1, HTTP/2, and WebSocket
        support out of the box, with battle-tested connection pooling and TLS.
      </p>

      <h2>The Desktop App</h2>
      <p>
        One thing that sets PortZero apart is the{" "}
        <a href="/docs/desktop-app">native desktop app</a>, built with{" "}
        <a
          href="https://tauri.app"
          target="_blank"
          rel="noopener noreferrer"
        >
          Tauri v2
        </a>
        . While the CLI gives you full control, the desktop app makes traffic
        inspection and mocking visual and immediate:
      </p>
      <ul>
        <li>See all running apps with CPU, memory, and uptime at a glance</li>
        <li>Browse and filter every HTTP request flowing through the proxy</li>
        <li>Create mock responses with a form instead of writing JSON</li>
        <li>Configure network simulation per-app with sliders</li>
        <li>Manage the daemon from the system tray</li>
      </ul>
      <p>
        This is the biggest gap I saw in tools like Portless — when you&apos;re
        debugging a tricky API interaction, you want to <em>see</em> the
        requests, not grep through logs. A visual dashboard turns traffic
        inspection from a chore into something you actually use.
      </p>

      <h2>Multi-App Config</h2>
      <p>
        For projects with multiple services, PortZero supports a{" "}
        <a href="/docs/configuration">
          <code>portzero.toml</code> config file
        </a>
        :
      </p>

      <CodeBlock
        lang="toml"
        filename="portzero.toml"
        code={`[proxy]
port = 1337
https = true

[apps.web]
command = "pnpm dev"
cwd = "./apps/web"

[apps.api]
command = "cargo run"
subdomain = "api"

[apps.worker]
command = "node worker.js"
subdomain = "worker"`}
      />

      <p>
        Run <code>portzero up</code> and all three services start with stable
        URLs. Run <code>portzero down</code> to stop them all. That&apos;s it.
      </p>

      <h2>What&apos;s Next</h2>
      <p>PortZero is open source and actively developed. Coming soon:</p>
      <ul>
        <li>
          <strong>Public tunnels</strong> — expose local apps to the internet
          with a single command, built on QUIC
        </li>
        <li>
          <strong>Windows support</strong> — currently macOS and Linux only
        </li>
        <li>
          <strong>API schema inference</strong> — automatically detect and
          document your API&apos;s shape from captured traffic
        </li>
        <li>
          <strong>Team sharing</strong> — share mock configurations and traffic
          snapshots with your team
        </li>
      </ul>

      <h2>Try It</h2>
      <CodeBlock
        lang="shellscript"
        code={`# Install
curl -fsSL https://goport0.dev/install.sh | bash

# Run your app
portzero my-app next dev

# Open http://my-app.localhost:1337`}
      />

      <p>
        Check out the{" "}
        <a
          href="https://github.com/portzero-dev/portzero"
          target="_blank"
          rel="noopener noreferrer"
        >
          GitHub repo
        </a>
        , read the{" "}
        <a href="/docs/getting-started">docs</a>, or{" "}
        <a href="/#download">download the desktop app</a>.
      </p>
    </div>
  );
}
