import Image from "next/image";
import {
  Zap,
  Eye,
  RefreshCw,
  Shield,
  Wifi,
  Terminal,
  Globe,
  MonitorSmartphone,
  ArrowRight,
  Github,
  ChevronRight,
  Download,
  Apple,
  Monitor,
} from "lucide-react";

function Navbar() {
  return (
    <nav className="fixed top-0 left-0 right-0 z-50 border-b border-zinc-800/50 bg-zinc-950/80 backdrop-blur-xl">
      <div className="mx-auto flex h-16 max-w-6xl items-center justify-between px-6">
        <div className="flex items-center gap-2">
          <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-violet-primary font-bold text-sm text-white">
            PZ
          </div>
          <span className="text-lg font-semibold">PortZero</span>
        </div>
        <div className="hidden items-center gap-8 md:flex">
          <a
            href="#features"
            className="text-sm text-zinc-400 transition-colors hover:text-white"
          >
            Features
          </a>
          <a
            href="#screenshots"
            className="text-sm text-zinc-400 transition-colors hover:text-white"
          >
            Screenshots
          </a>
          <a
            href="#quickstart"
            className="text-sm text-zinc-400 transition-colors hover:text-white"
          >
            Quick Start
          </a>
          <a
            href="#download"
            className="text-sm text-zinc-400 transition-colors hover:text-white"
          >
            Download
          </a>
          <a
            href="/docs"
            className="text-sm text-zinc-400 transition-colors hover:text-white"
          >
            Docs
          </a>
          <a
            href="/blog"
            className="text-sm text-zinc-400 transition-colors hover:text-white"
          >
            Blog
          </a>
          <a
            href="https://github.com/portzero-dev/portzero"
            target="_blank"
            rel="noopener noreferrer"
            className="text-sm text-zinc-400 transition-colors hover:text-white"
          >
            GitHub
          </a>
        </div>
        <div className="flex items-center gap-3">
          <a
            href="https://github.com/portzero-dev/portzero"
            target="_blank"
            rel="noopener noreferrer"
            className="flex items-center gap-2 rounded-lg border border-zinc-700 px-4 py-2 text-sm text-zinc-300 transition-colors hover:border-zinc-600 hover:text-white"
          >
            <Github className="h-4 w-4" />
            Star
          </a>
          <a
            href="#quickstart"
            className="rounded-lg bg-violet-primary px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-violet-hover"
          >
            Get Started
          </a>
        </div>
      </div>
    </nav>
  );
}

function Hero() {
  return (
    <section className="relative overflow-hidden pt-32 pb-20">
      {/* Background glow */}
      <div className="pointer-events-none absolute top-0 left-1/2 -translate-x-1/2">
        <div className="h-[500px] w-[800px] rounded-full bg-violet-primary/10 blur-[120px] animate-glow-pulse" />
      </div>

      <div className="relative mx-auto max-w-6xl px-6 text-center">
        <div className="animate-fade-in-up">
          <div className="mb-6 inline-flex items-center gap-2 rounded-full border border-zinc-800 bg-zinc-900/50 px-4 py-1.5 text-sm text-zinc-400">
            <Zap className="h-3.5 w-3.5 text-violet-primary" />
            Built with Rust &amp; Cloudflare Pingora
          </div>
        </div>

        <h1
          className="animate-fade-in-up mx-auto max-w-4xl text-5xl font-bold leading-tight tracking-tight sm:text-6xl lg:text-7xl"
          style={{ animationDelay: "0.1s" }}
        >
          Your local dev servers,{" "}
          <span className="bg-gradient-to-r from-violet-primary to-purple-400 bg-clip-text text-transparent">
            one URL away
          </span>
        </h1>

        <p
          className="animate-fade-in-up mx-auto mt-6 max-w-2xl text-lg text-zinc-400 leading-relaxed sm:text-xl"
          style={{ animationDelay: "0.2s" }}
        >
          PortZero assigns stable{" "}
          <code className="rounded bg-zinc-800 px-1.5 py-0.5 text-sm text-violet-primary">
            &lt;name&gt;.localhost
          </code>{" "}
          URLs to your dev servers, captures all HTTP traffic for inspection, and
          provides replay, mocking &amp; network simulation -- all from a single
          binary.
        </p>

        <div
          className="animate-fade-in-up mt-10 flex flex-col items-center justify-center gap-4 sm:flex-row"
          style={{ animationDelay: "0.3s" }}
        >
          <a
            href="#download"
            className="inline-flex items-center gap-2 rounded-xl bg-violet-primary px-6 py-3 text-base font-medium text-white transition-colors hover:bg-violet-hover"
          >
            <Download className="h-4 w-4" />
            Download Desktop App
          </a>
          <a
            href="https://github.com/portzero-dev/portzero"
            target="_blank"
            rel="noopener noreferrer"
            className="inline-flex items-center gap-2 rounded-xl border border-zinc-700 px-6 py-3 text-base font-medium text-zinc-300 transition-colors hover:border-zinc-600 hover:text-white"
          >
            <Github className="h-4 w-4" />
            View on GitHub
          </a>
        </div>

        {/* CLI install */}
        <div
          className="animate-fade-in-up mx-auto mt-10 max-w-xl"
          style={{ animationDelay: "0.35s" }}
        >
          <div className="flex flex-col items-center gap-3 sm:flex-row sm:justify-center">
            <div className="flex items-center gap-3 rounded-lg border border-zinc-800 bg-zinc-900/80 px-4 py-2.5">
              <Terminal className="h-4 w-4 shrink-0 text-zinc-500" />
              <code className="text-sm text-zinc-300">
                brew install portzero-dev/tap/portzero
              </code>
            </div>
          </div>
          <div className="mt-2 flex items-center justify-center gap-2 text-xs text-zinc-500">
            <span>or</span>
            <code className="rounded bg-zinc-800/50 px-1.5 py-0.5 text-zinc-400">
              curl -fsSL https://goport0.dev/install.sh | bash
            </code>
          </div>
        </div>

        {/* Terminal preview */}
        <div
          className="animate-fade-in-up mx-auto mt-16 max-w-2xl"
          style={{ animationDelay: "0.4s" }}
        >
          <div className="overflow-hidden rounded-xl border border-zinc-800 bg-zinc-900 shadow-2xl shadow-violet-primary/5">
            <div className="flex items-center gap-2 border-b border-zinc-800 px-4 py-3">
              <div className="h-3 w-3 rounded-full bg-red-500/80" />
              <div className="h-3 w-3 rounded-full bg-yellow-500/80" />
              <div className="h-3 w-3 rounded-full bg-green-500/80" />
              <span className="ml-2 text-xs text-zinc-500">Terminal</span>
            </div>
            <div className="p-6 font-mono text-sm leading-relaxed text-left">
              <div className="text-zinc-500">
                <span className="text-emerald-400">$</span> portzero next dev
              </div>
              <div className="mt-1 text-zinc-400">
                <span className="text-violet-primary">{"=>"}</span>{" "}
                http://my-app.localhost:1337
              </div>
              <div className="mt-4 text-zinc-500">
                <span className="text-emerald-400">$</span> portzero api cargo
                run
              </div>
              <div className="mt-1 text-zinc-400">
                <span className="text-violet-primary">{"=>"}</span>{" "}
                http://api.localhost:1337
              </div>
              <div className="mt-4 text-zinc-500">
                <span className="text-emerald-400">$</span> portzero list
              </div>
              <div className="mt-1 text-zinc-400">
                <span className="text-violet-primary">{"=>"}</span>{" "}
                my-app &nbsp;Running &nbsp;http://my-app.localhost:1337
              </div>
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}

const features: {
  icon: React.ElementType;
  title: string;
  description: string;
  color: string;
  comingSoon?: boolean;
}[] = [
  {
    icon: Zap,
    title: "Reverse Proxy",
    description:
      "Route <app>.localhost to local ports via Cloudflare Pingora. HTTP/1, HTTP/2, and WebSocket support out of the box.",
    color: "text-violet-primary",
  },
  {
    icon: Eye,
    title: "Traffic Inspector",
    description:
      "Full request/response capture with filtering, search, and SQLite persistence. Never miss a request again.",
    color: "text-blue-400",
  },
  {
    icon: RefreshCw,
    title: "Request Replay",
    description:
      "One-click re-send of captured requests with optional overrides. Side-by-side diff between original and replayed.",
    color: "text-emerald-400",
  },
  {
    icon: Shield,
    title: "Response Mocking",
    description:
      "Per-route synthetic responses without hitting upstream. Create mock rules via the desktop app or CLI.",
    color: "text-amber-400",
  },
  {
    icon: Wifi,
    title: "Network Simulation",
    description:
      "Inject latency, jitter, packet loss, and bandwidth throttling per-app. Test how your app handles degraded networks.",
    color: "text-red-400",
  },
  {
    icon: Terminal,
    title: "Process Manager",
    description:
      "Spawn, monitor, and auto-restart child processes with deterministic port assignment and live log streaming.",
    color: "text-cyan-400",
  },
  {
    icon: Globe,
    title: "Public Tunnels",
    description:
      "Coming soon -- Expose local apps to the internet with a single command. QUIC, WebSocket, and H2 transport.",
    color: "text-pink-400",
    comingSoon: true,
  },
  {
    icon: MonitorSmartphone,
    title: "Desktop App",
    description:
      "Native Tauri v2 dashboard with system tray. Manage apps, inspect traffic, create mocks, and simulate networks.",
    color: "text-teal-400",
  },
];

function Features() {
  return (
    <section id="features" className="py-24">
      <div className="mx-auto max-w-6xl px-6">
        <div className="text-center">
          <h2 className="text-3xl font-bold sm:text-4xl">
            Everything you need for local dev
          </h2>
          <p className="mx-auto mt-4 max-w-2xl text-lg text-zinc-400">
            A single binary that replaces half your dev toolbox. Built in Rust
            for speed.
          </p>
        </div>

        <div className="mt-16 grid gap-6 sm:grid-cols-2 lg:grid-cols-3">
          {features.map((feature) => (
            <div
              key={feature.title}
              className={`group relative rounded-xl border p-6 transition-colors ${
                feature.comingSoon
                  ? "border-zinc-800/60 bg-zinc-900/30 opacity-75"
                  : "border-zinc-800 bg-zinc-900/50 hover:border-zinc-700 hover:bg-zinc-900"
              }`}
            >
              {feature.comingSoon && (
                <span className="absolute top-4 right-4 rounded-full border border-zinc-700 bg-zinc-800 px-2.5 py-0.5 text-[10px] font-medium uppercase tracking-wider text-zinc-400">
                  Coming Soon
                </span>
              )}
              <div
                className={`inline-flex rounded-lg bg-zinc-800 p-2.5 ${feature.color}`}
              >
                <feature.icon className="h-5 w-5" />
              </div>
              <h3 className="mt-4 text-lg font-semibold">{feature.title}</h3>
              <p className="mt-2 text-sm leading-relaxed text-zinc-400">
                {feature.description}
              </p>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}

const screenshots = [
  {
    src: "/screenshots/dashboard.png",
    alt: "PortZero Dashboard -- Overview of running apps with live traffic feed",
    title: "Dashboard Overview",
    description:
      "See all running apps at a glance with port, uptime, CPU, memory stats and a live traffic feed.",
  },
  {
    src: "/screenshots/traffic.png",
    alt: "PortZero Traffic Inspector -- Full request/response capture with filtering",
    title: "Traffic Inspector",
    description:
      "Browse, filter, and search every HTTP request flowing through the proxy. Filter by app, method, or status.",
  },
  {
    src: "/screenshots/mocks.png",
    alt: "PortZero Mocks -- Create mock responses for API endpoints",
    title: "Response Mocking",
    description:
      "Create and manage mock responses directly from the desktop app. Set method, path, status, headers, and body.",
  },
  {
    src: "/screenshots/settings.png",
    alt: "PortZero Settings -- Daemon management, CLI install, and HTTPS certificates",
    title: "Settings",
    description:
      "Manage the proxy daemon, install the CLI tool, and trust HTTPS certificates -- all from one place.",
  },
];

function Screenshots() {
  return (
    <section id="screenshots" className="border-t border-zinc-800 py-24">
      <div className="mx-auto max-w-6xl px-6">
        <div className="text-center">
          <h2 className="text-3xl font-bold sm:text-4xl">
            See it in action
          </h2>
          <p className="mx-auto mt-4 max-w-2xl text-lg text-zinc-400">
            A native desktop app for managing your local dev environment.
            Inspect traffic, create mocks, and monitor apps.
          </p>
        </div>

        <div className="mt-16 space-y-16">
          {screenshots.map((shot, i) => (
            <div
              key={shot.src}
              className={`flex flex-col items-center gap-8 lg:flex-row ${
                i % 2 === 1 ? "lg:flex-row-reverse" : ""
              }`}
            >
              <div className="lg:w-2/3">
                <div className="overflow-hidden rounded-xl border border-zinc-800 shadow-2xl shadow-violet-primary/5">
                  <Image
                    src={shot.src}
                    alt={shot.alt}
                    width={1456}
                    height={816}
                    className="w-full"
                    quality={90}
                  />
                </div>
              </div>
              <div className="lg:w-1/3">
                <h3 className="text-xl font-semibold">{shot.title}</h3>
                <p className="mt-3 text-base leading-relaxed text-zinc-400">
                  {shot.description}
                </p>
              </div>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}

function QuickStart() {
  return (
    <section id="quickstart" className="border-t border-zinc-800 py-24">
      <div className="mx-auto max-w-6xl px-6">
        <div className="text-center">
          <h2 className="text-3xl font-bold sm:text-4xl">
            Up and running in seconds
          </h2>
          <p className="mx-auto mt-4 max-w-2xl text-lg text-zinc-400">
            Install, run, inspect. No configuration required.
          </p>
        </div>

        <div className="mx-auto mt-16 grid max-w-4xl gap-8 lg:grid-cols-3">
          <Step
            number={1}
            title="Install"
            code="curl -fsSL https://goport0.dev/install.sh | bash"
          />
          <Step
            number={2}
            title="Run your app"
            code="portzero next dev"
          />
          <Step
            number={3}
            title="Inspect traffic"
            code="# Open http://my-app.localhost:1337"
          />
        </div>

        {/* Multi-app config example */}
        <div className="mx-auto mt-16 max-w-2xl">
          <h3 className="mb-4 text-center text-lg font-semibold text-zinc-300">
            Or run multiple apps from a config file
          </h3>
          <div className="overflow-hidden rounded-xl border border-zinc-800 bg-zinc-900">
            <div className="flex items-center gap-2 border-b border-zinc-800 px-4 py-3">
              <span className="text-xs text-zinc-500">portzero.toml</span>
            </div>
            <pre className="overflow-x-auto p-6 font-mono text-sm leading-relaxed">
              <code>
                <span className="text-zinc-500">[proxy]</span>
                {"\n"}
                <span className="text-violet-primary">port</span>
                <span className="text-zinc-500"> = </span>
                <span className="text-emerald-400">1337</span>
                {"\n"}
                <span className="text-violet-primary">https</span>
                <span className="text-zinc-500"> = </span>
                <span className="text-emerald-400">true</span>
                {"\n\n"}
                <span className="text-zinc-500">[apps.web]</span>
                {"\n"}
                <span className="text-violet-primary">command</span>
                <span className="text-zinc-500"> = </span>
                <span className="text-amber-400">{'"pnpm dev"'}</span>
                {"\n"}
                <span className="text-violet-primary">cwd</span>
                <span className="text-zinc-500"> = </span>
                <span className="text-amber-400">{'"./apps/web"'}</span>
                {"\n\n"}
                <span className="text-zinc-500">[apps.api]</span>
                {"\n"}
                <span className="text-violet-primary">command</span>
                <span className="text-zinc-500"> = </span>
                <span className="text-amber-400">{'"cargo run"'}</span>
                {"\n"}
                <span className="text-violet-primary">subdomain</span>
                <span className="text-zinc-500"> = </span>
                <span className="text-amber-400">{'"api"'}</span>
              </code>
            </pre>
          </div>
        </div>
      </div>
    </section>
  );
}

function Step({
  number,
  title,
  code,
}: {
  number: number;
  title: string;
  code: string;
}) {
  return (
    <div className="text-center">
      <div className="mx-auto mb-4 flex h-10 w-10 items-center justify-center rounded-full border border-violet-primary/30 bg-violet-primary/10 text-sm font-bold text-violet-primary">
        {number}
      </div>
      <h3 className="text-lg font-semibold">{title}</h3>
      <div className="mt-3 overflow-hidden rounded-lg border border-zinc-800 bg-zinc-900 px-4 py-3">
        <code className="text-sm text-zinc-300">{code}</code>
      </div>
    </div>
  );
}

function CLI() {
  return (
    <section className="border-t border-zinc-800 py-24">
      <div className="mx-auto max-w-6xl px-6">
        <div className="grid items-center gap-12 lg:grid-cols-2">
          <div>
            <h2 className="text-3xl font-bold sm:text-4xl">
              Powerful CLI, zero config
            </h2>
            <p className="mt-4 text-lg leading-relaxed text-zinc-400">
              Every feature is accessible from the command line. Run apps,
              manage the daemon, tail logs, and more -- all with simple
              commands.
            </p>
            <a
              href="/docs/cli-reference"
              className="mt-6 inline-flex items-center gap-2 text-violet-primary transition-colors hover:text-violet-hover"
            >
              Read the full CLI reference
              <ChevronRight className="h-4 w-4" />
            </a>
          </div>
          <div className="overflow-hidden rounded-xl border border-zinc-800 bg-zinc-900 shadow-2xl">
            <div className="flex items-center gap-2 border-b border-zinc-800 px-4 py-3">
              <div className="h-3 w-3 rounded-full bg-red-500/80" />
              <div className="h-3 w-3 rounded-full bg-yellow-500/80" />
              <div className="h-3 w-3 rounded-full bg-green-500/80" />
            </div>
            <pre className="overflow-x-auto p-6 font-mono text-sm leading-relaxed text-zinc-400">
              <span className="text-zinc-500"># Run and manage apps</span>
              {"\n"}
              <span className="text-emerald-400">$</span> portzero next dev
              {"\n"}
              <span className="text-emerald-400">$</span> portzero my-api cargo
              run{"\n"}
              <span className="text-emerald-400">$</span> portzero list{"\n"}
              <span className="text-emerald-400">$</span> portzero logs
              my-app -f{"\n\n"}
              <span className="text-zinc-500"># Multi-app from config</span>
              {"\n"}
              <span className="text-emerald-400">$</span> portzero up{"\n"}
              <span className="text-emerald-400">$</span> portzero down{"\n\n"}
              <span className="text-zinc-500"># Daemon management</span>
              {"\n"}
              <span className="text-emerald-400">$</span> portzero start -d
              {"\n"}
              <span className="text-emerald-400">$</span> portzero status
            </pre>
          </div>
        </div>
      </div>
    </section>
  );
}

const DESKTOP_VERSION = "0.2.0";
const DESKTOP_RELEASE_URL = `https://github.com/portzero-dev/portzero/releases/download/desktop-v${DESKTOP_VERSION}`;

const downloads: {
  platform: string;
  icon: React.ElementType;
  builds: { label: string; href: string; badge?: string }[];
}[] = [
  {
    platform: "macOS",
    icon: Apple,
    builds: [
      {
        label: "Apple Silicon (M1+)",
        href: `${DESKTOP_RELEASE_URL}/PortZero_${DESKTOP_VERSION}_aarch64.dmg`,
        badge: "arm64",
      },
      {
        label: "Intel",
        href: `${DESKTOP_RELEASE_URL}/PortZero_${DESKTOP_VERSION}_x64.dmg`,
        badge: "x64",
      },
    ],
  },
  {
    platform: "Linux",
    icon: Monitor,
    builds: [
      {
        label: "AppImage",
        href: `${DESKTOP_RELEASE_URL}/PortZero_${DESKTOP_VERSION}_amd64.AppImage`,
        badge: "x64",
      },
      {
        label: "Debian / Ubuntu",
        href: `${DESKTOP_RELEASE_URL}/portzero_${DESKTOP_VERSION}_amd64.deb`,
        badge: ".deb",
      },
    ],
  },
];

function DesktopDownload() {
  return (
    <section id="download" className="border-t border-zinc-800 py-24">
      <div className="mx-auto max-w-6xl px-6">
        <div className="text-center">
          <h2 className="text-3xl font-bold sm:text-4xl">
            Download the Desktop App
          </h2>
          <p className="mx-auto mt-4 max-w-2xl text-lg text-zinc-400">
            A native app for managing apps, inspecting traffic, creating mocks,
            and simulating networks. Built with Tauri v2.
          </p>
        </div>

        <div className="mx-auto mt-16 grid max-w-3xl gap-6 sm:grid-cols-2">
          {downloads.map((platform) => (
            <div
              key={platform.platform}
              className="rounded-xl border border-zinc-800 bg-zinc-900/50 p-6 transition-colors hover:border-zinc-700 hover:bg-zinc-900"
            >
              <div className="flex items-center gap-3">
                <div className="inline-flex rounded-lg bg-zinc-800 p-2.5 text-violet-primary">
                  <platform.icon className="h-5 w-5" />
                </div>
                <h3 className="text-lg font-semibold">{platform.platform}</h3>
              </div>
              <div className="mt-5 space-y-3">
                {platform.builds.map((build) => (
                  <a
                    key={build.label}
                    href={build.href}
                    className="flex items-center justify-between rounded-lg border border-zinc-800 bg-zinc-950/50 px-4 py-3 text-sm text-zinc-300 transition-colors hover:border-violet-primary/40 hover:text-white"
                  >
                    <span className="flex items-center gap-2">
                      <Download className="h-4 w-4 text-zinc-500" />
                      {build.label}
                    </span>
                    {build.badge && (
                      <span className="rounded-full bg-zinc-800 px-2 py-0.5 text-[10px] font-medium uppercase tracking-wider text-zinc-400">
                        {build.badge}
                      </span>
                    )}
                  </a>
                ))}
              </div>
            </div>
          ))}
        </div>

        <p className="mt-8 text-center text-sm text-zinc-500">
          Windows support coming soon.{" "}
          <a
            href="https://github.com/portzero-dev/portzero/releases"
            target="_blank"
            rel="noopener noreferrer"
            className="text-violet-primary transition-colors hover:text-violet-hover"
          >
            View all releases on GitHub
          </a>
        </p>
      </div>
    </section>
  );
}

function CTA() {
  return (
    <section className="border-t border-zinc-800 py-24">
      <div className="mx-auto max-w-6xl px-6 text-center">
        <h2 className="text-3xl font-bold sm:text-4xl">
          Ready to simplify your local dev?
        </h2>
        <p className="mx-auto mt-4 max-w-xl text-lg text-zinc-400">
          Install PortZero and get stable URLs, traffic inspection, and more in
          under a minute.
        </p>
        <div className="mt-8 flex flex-col items-center justify-center gap-4 sm:flex-row">
          <a
            href="#quickstart"
            className="inline-flex items-center gap-2 rounded-xl bg-violet-primary px-6 py-3 text-base font-medium text-white transition-colors hover:bg-violet-hover"
          >
            Get Started
            <ArrowRight className="h-4 w-4" />
          </a>
          <a
            href="/docs"
            className="inline-flex items-center gap-2 rounded-xl border border-zinc-700 px-6 py-3 text-base font-medium text-zinc-300 transition-colors hover:border-zinc-600 hover:text-white"
          >
            Read the Docs
          </a>
        </div>
      </div>
    </section>
  );
}

function Footer() {
  return (
    <footer className="border-t border-zinc-800 py-12">
      <div className="mx-auto max-w-6xl px-6">
        <div className="flex flex-col items-center justify-between gap-6 sm:flex-row">
          <div className="flex items-center gap-2">
            <div className="flex h-7 w-7 items-center justify-center rounded-md bg-violet-primary text-xs font-bold text-white">
              PZ
            </div>
            <span className="text-sm font-semibold">PortZero</span>
          </div>
          <div className="flex items-center gap-6 text-sm text-zinc-500">
            <a
              href="/docs"
              className="transition-colors hover:text-zinc-300"
            >
              Docs
            </a>
            <a
              href="/blog"
              className="transition-colors hover:text-zinc-300"
            >
              Blog
            </a>
            <a
              href="https://github.com/portzero-dev/portzero"
              target="_blank"
              rel="noopener noreferrer"
              className="transition-colors hover:text-zinc-300"
            >
              GitHub
            </a>
            <span>MIT / Apache-2.0</span>
          </div>
        </div>
      </div>
    </footer>
  );
}

export default function Home() {
  return (
    <>
      <Navbar />
      <main>
        <Hero />
        <Screenshots />
        <Features />
        <QuickStart />
        <CLI />
        <DesktopDownload />
        <CTA />
      </main>
      <Footer />
    </>
  );
}
