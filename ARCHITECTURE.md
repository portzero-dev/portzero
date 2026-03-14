# PortZero Architecture

## 1. Overview

PortZero is a local development reverse proxy, process manager, and traffic inspector built in **Rust** using **Cloudflare Pingora** as the proxy engine and **Tauri v2** for the desktop dashboard. It assigns stable `<name>.localhost` URLs to dev servers, captures all HTTP traffic for inspection, and provides request replay, mocking, interception, and network simulation -- all from a single binary.

```
portzero <command>                  # name inferred from cwd directory name
portzero <my-app> <command>         # explicit name
```

PortZero launches `<command>` with a `PORT` env variable, then routes all traffic from `<my-app>.localhost` to that port.

---

## 2. Competitive Analysis: Portless vs PortZero

Portless (vercel-labs/portless) is a Node.js proxy that solves the port-naming problem. PortZero differentiates on three axes: **performance** (Rust + Pingora vs Node.js), **observability** (traffic capture, replay, mocking), and **developer experience** (native desktop app, MCP server, network simulation).

| Capability | Portless | PortZero |
|---|---|---|
| **Runtime** | Node.js | **Rust (Pingora)** |
| **Proxy performance** | Single-threaded event loop | **Multi-threaded async (tokio), battle-tested at Cloudflare scale** |
| Subdomain routing | Yes | Yes |
| Auto PORT assignment | Yes (random 4000-4999) | Yes (deterministic hash + fallback) |
| HTTPS / HTTP/2 | Yes (auto-cert) | Yes (auto-cert via rcgen + rustls) |
| WebSocket proxying | Yes | Yes (Pingora native upgrade support) |
| **Desktop app** | No | **Yes -- Tauri v2 native app with system tray** |
| **Web dashboard fallback** | No | **Yes -- same SPA served by daemon** |
| **Request inspector** | No | **Yes -- full req/res capture with body** |
| **Request filtering** | No | **Yes -- by app, status, method, path, full-text** |
| **Request replay** | No | **Yes -- one-click re-send with overrides** |
| **Request interception** | No | **Yes -- pause, inspect, edit, forward/drop** |
| **Response mocking** | No | **Yes -- per-route mock responses** |
| **Network simulation** | No | **Yes -- latency injection, packet loss, throttling** |
| **Request diffing** | No | **Yes -- side-by-side comparison** |
| **Auto API schema** | No | **Yes -- passive OpenAPI inference from traffic** |
| **MCP server** | No | **Yes -- AI agents can inspect traffic and manage apps** |
| **Public tunnels** | No | **Yes -- `portzero share` via LocalUp (QUIC/WS/H2, self-hostable)** |
| **Process health monitoring** | No (PID check only) | **Yes -- uptime, restarts, CPU/mem** |
| **Auto-restart on crash** | No | **Yes -- configurable with backoff** |
| **Live log streaming** | No | **Yes -- per-app in dashboard** |
| **Stdout URL rewriting** | No | **Yes -- rewrites port URLs to .localhost in output** |
| **Config file support** | No | **Yes -- `portzero.toml`** |
| **API for tooling** | No | **Yes -- REST + WebSocket** |
| Daemon architecture | File-based PID + routes.json | **Pingora Server with graceful reload** |
| State management | JSON files with dir-lock | **SQLite via rusqlite (WAL mode)** |
| Binary size | ~2MB (node_modules) | **~8MB single static binary** |
| Install | `npm install -g` | **Single binary download or `cargo install`** |

---

## 3. High-Level Architecture

```
                          Browser / curl / AI agent
                                 |
                    *.localhost:1337 (single port)
                                 |
                   +─────────────┴──────────────+
                   |     PortZero Daemon         |
                   |     (Rust / Pingora)         |
                   |                             |
                   |  ┌───────────────────────┐  |
                   |  │   Pingora ProxyHttp   │  |  HTTP/1.1, HTTP/2, WebSocket
                   |  │   (reverse proxy)     │  |
                   |  └──────┬────────┬──────┘  |
                   |         │        │         |
                   |  ┌──────┴──┐  ┌──┴──────┐  |
                   |  │ Router  │  │Recorder │  |  Subdomain → port
                   |  └──────┬──┘  └──┬──────┘  |  Req/res → SQLite
                   |         │        │         |
                   |  ┌──────┴──┐  ┌──┴──────┐  |
                   |  │Intercept│  │  Mock   │  |  Breakpoints + mock engine
                   |  │  Engine │  │ Engine  │  |
                   |  └─────────┘  └─────────┘  |
                   |         │        │         |
                   |  ┌──────┴────────┴──────┐  |
                   |  │   Process Manager    │  |  Spawn/monitor child processes
                   |  └──────────────────────┘  |
                   |         │        │         |
                   |  ┌──────┴──┐  ┌──┴──────┐  |
                   |  │  API    │  │  MCP    │  |  REST + WS for dashboard
                   |  │ Server  │  │ Server  │  |  stdio for AI agents
                   |  └─────────┘  └─────────┘  |
                   +─────────────────────────────+
                       │        │         │
              :4001 app-a   :4002 app-b   Tauri Desktop App
              (next dev)    (vite dev)    (React dashboard)

              Web fallback: _portzero.localhost:1337
```

---

## 4. Project Structure

```
portzero/
├── Cargo.toml                         # Workspace root
├── Cargo.lock
│
├── crates/
│   ├── portzero-core/                 # Core library (no binary, no UI)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── proxy.rs               # Pingora ProxyHttp implementation
│   │       ├── router.rs              # Subdomain → port routing table
│   │       ├── recorder.rs            # Request/response capture + SQLite storage
│   │       ├── interceptor.rs         # Request breakpoints (pause/edit/forward)
│   │       ├── mock_engine.rs         # Response mocking per-route
│   │       ├── network_sim.rs         # Latency/loss/throttle injection
│   │       ├── process_manager.rs     # Child process spawn/monitor/restart
│   │       ├── schema_inference.rs    # Passive OpenAPI schema builder
│   │       ├── store.rs               # SQLite persistence (rusqlite)
│   │       ├── tunnel.rs              # Public tunnel via localup-lib
│   │       ├── certs.rs               # TLS cert generation (rcgen + rustls)
│   │       ├── config.rs              # portzero.toml loader
│   │       └── types.rs               # Shared types
│   │
│   ├── portzero-cli/                  # CLI binary
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs                # Argument parsing (clap), command dispatch
│   │       ├── daemon.rs              # Daemon start/stop/foreground
│   │       └── commands/
│   │           ├── mod.rs
│   │           ├── run.rs             # portzero [name] <command>
│   │           ├── up.rs              # portzero up (from config)
│   │           ├── list.rs            # portzero list
│   │           ├── logs.rs            # portzero logs <name>
│   │           ├── share.rs           # portzero share <name>
│   │           ├── mock.rs            # portzero mock <name> ...
│   │           ├── intercept.rs       # portzero intercept <name> ...
│   │           └── throttle.rs        # portzero throttle <name> ...
│   │
│   ├── portzero-api/                  # HTTP API + WebSocket server
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── server.rs              # axum HTTP server
│   │       ├── routes/
│   │       │   ├── mod.rs
│   │       │   ├── apps.rs            # /api/apps endpoints
│   │       │   ├── requests.rs        # /api/requests endpoints
│   │       │   ├── mocks.rs           # /api/mocks endpoints
│   │       │   ├── intercepts.rs      # /api/intercepts endpoints
│   │       │   ├── schema.rs          # /api/apps/:name/schema
│   │       │   ├── tunnel.rs          # /api/apps/:name/share
│   │       │   └── status.rs          # /api/status
│   │       ├── ws.rs                  # WebSocket hub (tokio broadcast)
│   │       └── static_files.rs        # Serve embedded dashboard SPA
│   │
│   ├── portzero-mcp/                  # MCP server (Model Context Protocol)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       └── tools.rs               # MCP tool definitions
│   │
apps/
├── desktop/                           # Tauri v2 desktop app
│   ├── src-tauri/
│   │   ├── Cargo.toml                 # Tauri Rust backend
│   │   ├── tauri.conf.json
│   │   └── src/
│   │       ├── main.rs                # Tauri app entry point
│   │       ├── tray.rs                # System tray menu + actions
│   │       ├── commands.rs            # Tauri IPC commands
│   │       └── daemon_bridge.rs       # Manage daemon lifecycle from app
│   └── src/                           # React frontend (Vite)
│           ├── package.json
│           ├── vite.config.ts
│           ├── tailwind.config.ts
│           ├── index.html
│           └── src/
│               ├── main.tsx
│               ├── App.tsx
│               ├── api/
│               │   ├── client.ts      # HTTP API client
│               │   └── useWebSocket.ts
│               ├── pages/
│               │   ├── Overview.tsx
│               │   ├── Traffic.tsx
│               │   ├── RequestDetail.tsx
│               │   ├── AppDetail.tsx
│               │   ├── Mocks.tsx
│               │   └── Intercepts.tsx
│               ├── components/
│               │   ├── AppCard.tsx
│               │   ├── RequestRow.tsx
│               │   ├── FilterBar.tsx
│               │   ├── ReplayButton.tsx
│               │   ├── DiffViewer.tsx
│               │   ├── MockEditor.tsx
│               │   ├── InterceptModal.tsx
│               │   ├── NetworkSimPanel.tsx
│               │   ├── SchemaViewer.tsx
│               │   ├── LogViewer.tsx
│               │   ├── TunnelStatus.tsx
│               │   └── StatusBadge.tsx
│               └── lib/
│                   ├── filters.ts
│                   └── formatters.ts
│
├── portzero.toml                      # Example config file
└── README.md
```

---

## 5. Component Design

### 5.1 Proxy Engine (`portzero-core/proxy.rs`)

The proxy is built on **Pingora's `ProxyHttp` trait**. Pingora gives us HTTP/1.1, HTTP/2, WebSocket upgrades, connection pooling, TLS, and graceful reload for free. The `ProxyHttp` trait provides callbacks at every stage of the request lifecycle, which we use to wire in recording, interception, mocking, and network simulation.

```rust
use async_trait::async_trait;
use pingora::prelude::*;
use pingora_proxy::{ProxyHttp, Session};

pub struct PortZeroProxy {
    router: Arc<Router>,
    recorder: Arc<Recorder>,
    interceptor: Arc<Interceptor>,
    mock_engine: Arc<MockEngine>,
    network_sim: Arc<NetworkSim>,
    ws_hub: Arc<WsHub>,
}

/// Per-request context threaded through all Pingora callbacks.
pub struct RequestContext {
    app_name: String,
    target_port: u16,
    recording_id: String,
    request_start: Instant,
    captured_request: CapturedRequest,
    intercepted: bool,
    mocked: bool,
}

#[async_trait]
impl ProxyHttp for PortZeroProxy {
    type CTX = RequestContext;

    fn new_ctx(&self) -> Self::CTX {
        RequestContext::default()
    }

    /// Route based on subdomain: my-app.localhost → port 4001
    async fn upstream_peer(
        &self,
        session: &mut Session,
        ctx: &mut Self::CTX,
    ) -> Result<Box<HttpPeer>> {
        let host = session.req_header()
            .headers.get("host")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("");

        let subdomain = extract_subdomain(host);

        // Check for reserved subdomains
        if subdomain == "_portzero" {
            // Handled separately by the API server
            return Err(Error::explain(
                HTTPStatus(404),
                "reserved subdomain",
            ));
        }

        let route = self.router.resolve(&subdomain)
            .ok_or_else(|| Error::explain(HTTPStatus(404), "no app registered"))?;

        ctx.app_name = subdomain.to_string();
        ctx.target_port = route.port;
        ctx.recording_id = uuid::Uuid::new_v4().to_string();
        ctx.request_start = Instant::now();

        // Capture request metadata
        ctx.captured_request = CapturedRequest::from_session(session);

        // Broadcast request:start via WebSocket
        self.ws_hub.broadcast(Event::RequestStart {
            id: &ctx.recording_id,
            app: &ctx.app_name,
            method: session.req_header().method.as_str(),
            url: session.req_header().uri.path(),
        });

        Ok(Box::new(HttpPeer::new(
            format!("127.0.0.1:{}", route.port),
            false, // no TLS to upstream (local process)
            String::new(),
        )))
    }

    /// Intercept + mock check before forwarding to upstream.
    async fn request_filter(
        &self,
        session: &mut Session,
        ctx: &mut Self::CTX,
    ) -> Result<bool> {
        // 1. Check if a mock matches this request
        if let Some(mock_response) = self.mock_engine.match_request(
            &ctx.app_name,
            session.req_header(),
        ) {
            ctx.mocked = true;
            let resp = mock_response.to_pingora_header();
            session.write_response_header(Box::new(resp), false).await?;
            session.write_response_body(
                Some(mock_response.body.into()),
                true,
            ).await?;
            // Record the mocked response
            self.recorder.record_mock(&ctx.recording_id, &mock_response);
            return Ok(true); // response already sent
        }

        // 2. Check if an intercept breakpoint matches
        if self.interceptor.should_intercept(&ctx.app_name, session.req_header()) {
            ctx.intercepted = true;
            // Pause and wait for dashboard user to decide
            let decision = self.interceptor
                .pause_and_wait(&ctx.recording_id, session)
                .await;

            match decision {
                InterceptDecision::Forward => { /* continue normally */ }
                InterceptDecision::ForwardModified(modified) => {
                    // Apply modifications to request headers/body
                    modified.apply_to(session);
                }
                InterceptDecision::Drop => {
                    session.respond_error(444).await?; // connection closed
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    /// Inject network simulation delays before upstream connection.
    async fn upstream_request_filter(
        &self,
        session: &mut Session,
        upstream_request: &mut RequestHeader,
        ctx: &mut Self::CTX,
    ) -> Result<()> {
        // Apply latency injection
        if let Some(delay) = self.network_sim.get_delay(&ctx.app_name) {
            tokio::time::sleep(delay).await;
        }

        // Apply packet loss simulation
        if self.network_sim.should_drop(&ctx.app_name) {
            return Err(Error::explain(
                ConnectTimedout,
                "simulated network failure",
            ));
        }

        // Rewrite stdout-detected port in Host header
        upstream_request.insert_header("Host", &format!("localhost:{}", ctx.target_port))?;

        Ok(())
    }

    /// Capture response headers for recording.
    async fn upstream_response_filter(
        &self,
        session: &mut Session,
        upstream_response: &mut ResponseHeader,
        ctx: &mut Self::CTX,
    ) -> Result<()> {
        ctx.captured_request.response_status = upstream_response.status.as_u16();
        ctx.captured_request.response_headers = headers_to_map(upstream_response);
        Ok(())
    }

    /// Capture response body chunks.
    fn upstream_response_body_filter(
        &self,
        session: &mut Session,
        body: &mut Option<Bytes>,
        end_of_stream: bool,
        ctx: &mut Self::CTX,
    ) -> Result<Option<Duration>> {
        if let Some(data) = body {
            ctx.captured_request.append_response_body(data);
        }
        Ok(None)
    }

    /// Final logging: persist to SQLite, broadcast completion.
    async fn logging(
        &self,
        session: &mut Session,
        error: Option<&Error>,
        ctx: &mut Self::CTX,
    ) {
        let duration = ctx.request_start.elapsed();

        // Persist to SQLite
        self.recorder.save(RequestRecord {
            id: ctx.recording_id.clone(),
            app_name: ctx.app_name.clone(),
            timestamp: SystemTime::now(),
            duration,
            captured: ctx.captured_request.clone(),
            mocked: ctx.mocked,
            intercepted: ctx.intercepted,
        });

        // Feed to schema inference engine
        self.schema_inference.observe(&ctx.app_name, &ctx.captured_request);

        // Broadcast request:complete via WebSocket
        self.ws_hub.broadcast(Event::RequestComplete {
            id: &ctx.recording_id,
            app: &ctx.app_name,
            status: ctx.captured_request.response_status,
            duration_ms: duration.as_millis() as u64,
        });
    }
}
```

**Why Pingora over hand-rolled `hyper`:**

| Concern | hyper | Pingora |
|---|---|---|
| HTTP/2 downstream + HTTP/1.1 upstream | Manual wiring | Built-in |
| WebSocket upgrades | Manual `upgrade()` | Native support |
| Connection pooling to upstreams | Manual | Built-in with health checks |
| Graceful restart (zero downtime) | Not provided | `Server::run_forever()` + SIGQUIT |
| Request lifecycle hooks | Not provided | `ProxyHttp` trait with 15+ callbacks |
| Battle-tested at scale | Library | Serves 40M req/s at Cloudflare |
| Daemonization | Manual | Built-in `-d` flag |

### 5.2 Router (`portzero-core/router.rs`)

Thread-safe routing table behind `Arc<RwLock<>>`. Read-heavy workload (every request reads, rare writes on app register/deregister).

```rust
pub struct Router {
    routes: RwLock<HashMap<String, Route>>,
}

pub struct Route {
    pub hostname: String,       // "my-app" (without .localhost)
    pub port: u16,              // 4001
    pub pid: u32,               // OS process ID
    pub command: Vec<String>,   // ["next", "dev"]
    pub cwd: PathBuf,           // Working directory
    pub started_at: SystemTime,
    pub status: AppStatus,
}

#[derive(Clone, Debug)]
pub enum AppStatus {
    Running,
    Crashed { exit_code: i32, at: SystemTime },
    Stopped,
}
```

Nested subdomains (`api.my-app.localhost`) resolve by longest-suffix match: `api.my-app` is checked first, then `my-app`.

### 5.3 Recorder (`portzero-core/recorder.rs`)

Captures full HTTP request/response pairs into SQLite for dashboard queries.

```rust
pub struct Recorder {
    db: Arc<Mutex<Connection>>,  // rusqlite
}

pub struct RequestRecord {
    pub id: String,
    pub app_name: String,
    pub timestamp: SystemTime,
    pub duration: Duration,

    // Request
    pub method: String,
    pub url: String,
    pub path: String,
    pub query_string: String,
    pub request_headers: HashMap<String, String>,
    pub request_body: Option<Vec<u8>>,
    pub request_content_type: Option<String>,

    // Response
    pub status_code: u16,
    pub status_message: String,
    pub response_headers: HashMap<String, String>,
    pub response_body: Option<Vec<u8>>,
    pub response_content_type: Option<String>,

    // Metadata
    pub mocked: bool,
    pub intercepted: bool,
    pub parent_id: Option<String>,  // For replayed requests
}
```

**Storage strategy:**
- SQLite with WAL mode via `rusqlite`
- Last 10,000 requests retained (configurable, FIFO eviction)
- Bodies capped at 1MB, stored as BLOBs
- Headers stored as JSON TEXT columns
- Indexed on: `app_name`, `timestamp`, `status_code`, `method`, `path`
- `r2d2` connection pool for concurrent reads from API handlers

### 5.4 Interceptor (`portzero-core/interceptor.rs`)

Pauses matching requests, sends them to the dashboard for human review, then forwards/drops based on the decision.

```rust
pub struct Interceptor {
    /// Active breakpoint rules
    rules: RwLock<Vec<InterceptRule>>,
    /// Pending intercepts waiting for dashboard decision
    pending: DashMap<String, PendingIntercept>,
}

pub struct InterceptRule {
    pub id: String,
    pub app_name: String,
    pub method: Option<String>,         // "POST", "GET", etc.
    pub path_pattern: Option<String>,   // "/api/webhook*"
    pub enabled: bool,
}

struct PendingIntercept {
    request_snapshot: CapturedRequest,
    decision_tx: oneshot::Sender<InterceptDecision>,
}

pub enum InterceptDecision {
    Forward,
    ForwardModified(RequestModification),
    Drop,
}
```

**Flow:**
1. Proxy calls `interceptor.should_intercept()` during `request_filter`
2. If matched, creates a `PendingIntercept` with a `tokio::sync::oneshot` channel
3. Broadcasts `intercept:pending` event via WebSocket to dashboard
4. Dashboard shows modal: "Incoming POST /api/webhook -- [Edit & Forward] [Forward] [Drop]"
5. User decision hits `POST /api/intercepts/:id/decide`, which sends on the oneshot channel
6. Proxy callback resumes with the decision
7. Configurable timeout (default 60s) auto-forwards if no decision is made

### 5.5 Mock Engine (`portzero-core/mock_engine.rs`)

Returns synthetic responses for matching requests without hitting the upstream app.

```rust
pub struct MockEngine {
    mocks: RwLock<Vec<MockRule>>,
}

pub struct MockRule {
    pub id: String,
    pub app_name: String,
    pub method: Option<String>,
    pub path_pattern: String,           // "/api/payments"
    pub status_code: u16,               // 500
    pub response_headers: HashMap<String, String>,
    pub response_body: String,          // JSON string
    pub enabled: bool,
    pub hit_count: AtomicU64,
}
```

Mocks are evaluated in `request_filter` before the request is proxied. If a mock matches, the response is written directly and `Ok(true)` is returned (response already sent).

**CLI interface:**
```
portzero mock my-app POST /api/payments --status 500 --body '{"error":"declined"}'
portzero mock my-app GET "/api/users/*" --status 200 --body-file ./fixtures/users.json
portzero mock list
portzero mock disable <id>
```

**Dashboard interface:** Click any captured request → "Mock this response" → edit status/headers/body → toggle on/off.

### 5.6 Network Simulation (`portzero-core/network_sim.rs`)

Injects realistic network conditions at the proxy layer. Applied in `upstream_request_filter` (before connection) and `upstream_response_body_filter` (for throttling).

```rust
pub struct NetworkSim {
    profiles: RwLock<HashMap<String, NetworkProfile>>,
}

pub struct NetworkProfile {
    pub app_name: String,
    pub latency: Option<Duration>,              // Fixed delay added to every response
    pub latency_jitter: Option<Duration>,        // Random +/- range
    pub packet_loss_rate: f64,                   // 0.0-1.0, probability of dropping request
    pub bandwidth_limit: Option<u64>,            // Bytes/sec throttle on response body
    pub path_filter: Option<String>,             // Only apply to matching paths
}
```

**CLI interface:**
```
portzero throttle my-app --latency 2000ms
portzero throttle my-app --path "/api/*" --drop 0.1
portzero throttle my-app --bandwidth 50kb/s
portzero throttle clear my-app
```

**Dashboard interface:** "Network Conditions" panel with sliders for latency, loss rate, and bandwidth per app.

### 5.7 Schema Inference (`portzero-core/schema_inference.rs`)

Passively builds an OpenAPI-like schema by observing traffic. Zero config.

```rust
pub struct SchemaInference {
    schemas: RwLock<HashMap<String, InferredSchema>>,
}

pub struct InferredSchema {
    pub app_name: String,
    pub endpoints: Vec<InferredEndpoint>,
    pub last_updated: SystemTime,
}

pub struct InferredEndpoint {
    pub method: String,
    pub path_template: String,           // "/api/users/:id" (parameterized)
    pub query_params: Vec<ParamInfo>,
    pub request_body_schema: Option<JsonSchema>,
    pub response_schemas: HashMap<u16, JsonSchema>,  // status_code → schema
    pub sample_count: u64,
}
```

**How it works:**
1. On every `logging` callback, `schema_inference.observe()` is called
2. Paths are parameterized: `/api/users/123` + `/api/users/456` → `/api/users/:id`
3. JSON bodies are analyzed to infer field types, required fields, enums
4. Multiple observations are merged (union of fields, narrowing of types)
5. Available at `GET /api/apps/:name/schema` and in the dashboard

### 5.8 Process Manager (`portzero-core/process_manager.rs`)

Spawns, monitors, and optionally restarts child processes.

```rust
pub struct ProcessManager {
    processes: DashMap<String, ManagedProcess>,
}

pub struct ManagedProcess {
    pub name: String,
    pub pid: u32,
    pub port: u16,
    pub command: Vec<String>,
    pub cwd: PathBuf,
    pub status: AppStatus,
    pub started_at: SystemTime,
    pub restarts: u32,
    pub auto_restart: bool,
    pub log_buffer: VecDeque<LogLine>,   // Ring buffer, last 5000 lines
    child: Child,                        // tokio::process::Child
    stdout_tx: broadcast::Sender<LogLine>,
}

pub struct LogLine {
    pub timestamp: SystemTime,
    pub stream: LogStream,               // Stdout | Stderr
    pub content: String,
}
```

**Features:**
- **Deterministic port assignment**: `(hash(app_name) % 1000) + 4000`, with linear probe on collision
- **Auto-restart**: Configurable per-app. Exponential backoff: 1s, 2s, 4s, 8s, ... max 30s. Resets after 60s of stability.
- **Log capture**: stdout/stderr split and captured into a ring buffer. Also forwarded to the user's terminal with prefix.
- **Stdout URL rewriting**: Scans stdout lines for patterns like `http://localhost:4001` and rewrites to `http://my-app.localhost:1337` so the developer sees correct clickable URLs.
- **Graceful shutdown**: SIGTERM → wait 5s → SIGKILL
- **Resource monitoring**: Periodic `/proc/<pid>/stat` reads (Linux) or `proc_pidinfo` (macOS) for CPU/memory

### 5.9 Tunnel via LocalUp (`portzero-core/tunnel.rs`)

Exposes a local app to the internet via **[LocalUp](https://github.com/localup-dev/localup)**, a Rust-based geo-distributed tunnel system. LocalUp is integrated as a native Rust dependency (`localup-lib` crate), not a sidecar binary -- meaning zero extra processes and a seamless API.

Traffic that comes through the tunnel still flows through PortZero's Pingora proxy, so all requests are recorded, filterable, and replayable in the dashboard just like local traffic.

```rust
use localup_lib::{TunnelClient, TunnelConfig, ExitNodeConfig, ProtocolConfig};

pub struct TunnelManager {
    active_tunnels: DashMap<String, TunnelHandle>,
    relay_url: String,          // Configurable relay server
    auth_token: Option<String>, // JWT token for relay auth
}

pub struct TunnelHandle {
    pub app_name: String,
    pub public_url: String,             // "https://my-app.relay.example.com"
    pub relay: String,                  // Relay server used
    pub transport: TransportProtocol,   // QUIC, WebSocket, or H2
    pub started_at: SystemTime,
    client: TunnelClient,               // localup-lib client handle
}

impl TunnelManager {
    /// Start a tunnel for a registered app.
    /// Connects to the LocalUp relay and exposes the app's port.
    pub async fn share(
        &self,
        app_name: &str,
        subdomain: Option<&str>,
        port: u16,
    ) -> Result<TunnelHandle> {
        let subdomain = subdomain
            .unwrap_or(app_name)
            .to_string();

        let config = TunnelConfig {
            local_port: Some(port),
            local_host: "127.0.0.1".to_string(),
            exit_node: ExitNodeConfig::Custom(self.relay_url.clone()),
            subdomain: Some(subdomain.clone()),
            protocol: ProtocolConfig::Https,
            token: self.auth_token.clone(),
            ..Default::default()
        };

        let client = TunnelClient::connect(config).await?;
        let public_url = client.public_url()
            .ok_or_else(|| anyhow!("tunnel connected but no URL assigned"))?
            .to_string();

        let handle = TunnelHandle {
            app_name: app_name.to_string(),
            public_url: public_url.clone(),
            relay: self.relay_url.clone(),
            transport: TransportProtocol::Quic, // auto-detected by localup
            started_at: SystemTime::now(),
            client,
        };

        self.active_tunnels.insert(app_name.to_string(), handle);
        Ok(handle)
    }

    /// Stop a tunnel for an app.
    pub async fn unshare(&self, app_name: &str) -> Result<()> {
        if let Some((_, handle)) = self.active_tunnels.remove(app_name) {
            drop(handle.client); // TunnelClient disconnects on drop
        }
        Ok(())
    }
}
```

**Why LocalUp over alternatives:**

| Concern | cloudflared | bore | LocalUp |
|---|---|---|---|
| Language | Go (sidecar binary) | Rust (library) | **Rust (library)** |
| Integration | Subprocess | Embedded | **Embedded via `localup-lib`** |
| Transports | Proprietary | TCP only | **QUIC + WebSocket + HTTP/2** |
| Auth | Cloudflare account | None | **JWT (self-hosted)** |
| Self-hostable relay | No | Yes | **Yes (geo-distributed)** |
| HTTPS support | Yes (Cloudflare) | No | **Yes (auto-cert via Let's Encrypt)** |
| Protocol discovery | No | No | **Yes (`/.well-known/localup-protocols`)** |
| TCP tunnels | Yes | Yes | **Yes (port-based routing)** |

**CLI:**
```
portzero share my-app
# → https://my-app.relay.example.com
# Press Ctrl+C to stop sharing

portzero share my-app --subdomain custom-name
# → https://custom-name.relay.example.com

portzero share my-app --relay my-relay.example.com:4443
# → Use a specific relay server
```

**Configuration** (in `portzero.toml`):
```toml
[tunnel]
relay = "relay.portzero.dev:4443"   # Default relay server
token = "your-jwt-token"            # Or set PORTZERO_TUNNEL_TOKEN env var

# Or self-host your own relay:
# relay = "my-relay.example.com:4443"
```

**Dashboard integration:** The tunnel status appears in the app detail page. When a tunnel is active, all requests through it show a "tunnel" badge in the traffic view, and the public URL is displayed prominently.

### 5.10 MCP Server (`portzero-mcp/`)

Exposes PortZero as an **MCP (Model Context Protocol) server** so AI coding agents can programmatically inspect traffic, manage apps, replay requests, and read logs. This is the highest-impact differentiator -- it turns PortZero into an AI agent's debugger.

```rust
pub struct PortZeroMcpServer {
    store: Arc<Store>,
    process_manager: Arc<ProcessManager>,
    recorder: Arc<Recorder>,
    router: Arc<Router>,
}
```

**MCP Tools exposed:**

| Tool | Description |
|---|---|
| `list_apps` | List all running apps with status, port, URL |
| `get_app_logs` | Get last N log lines for an app |
| `get_recent_requests` | Get last N requests, optionally filtered by app/status/method |
| `get_request_detail` | Get full request/response for a specific request ID |
| `replay_request` | Re-send a captured request with optional overrides |
| `restart_app` | Restart a crashed or running app |
| `get_app_schema` | Get the inferred API schema for an app |

**How agents use it:**

An AI agent debugging "why does the signup form fail?" can:
1. Call `list_apps` → sees `web` and `api` are running
2. Call `get_recent_requests(app="api", status_range="5xx", limit=5)` → sees `POST /api/users` returned 500
3. Call `get_request_detail(id="abc-123")` → sees the full error response body
4. Read the relevant source code, fix the bug
5. Call `replay_request(id="abc-123")` → confirms it now returns 201

**Transport**: stdio (standard MCP transport). Configured in the agent's MCP settings:

```json
{
  "mcpServers": {
    "portzero": {
      "command": "portzero",
      "args": ["mcp"]
    }
  }
}
```

---

### 5.11 API Server (`portzero-api/`)

Built on **axum** (Tokio-native, fast, ergonomic). Served on the reserved `_portzero.localhost` subdomain and used by both the Tauri app and the web fallback dashboard.

#### Endpoints

```
# Apps
GET    /api/apps                          # List all connected apps
GET    /api/apps/:name                    # Single app details + resource usage
POST   /api/apps/:name/restart            # Restart an app
POST   /api/apps/:name/stop               # Stop an app
GET    /api/apps/:name/logs               # Get buffered logs (?lines=100)
GET    /api/apps/:name/schema             # Get inferred API schema

# Traffic
GET    /api/requests                      # List captured requests
       ?app=my-app                        #   Filter by app
       &method=POST                       #   Filter by method
       &status=500                        #   Filter by exact status
       &status_range=4xx                  #   Filter by status range
       &path=/api/users                   #   Filter by path prefix
       &search=error                      #   Full-text search
       &from=1708300000000                #   Timestamp range
       &to=1708400000000
       &limit=50&offset=0                 #   Pagination
GET    /api/requests/:id                  # Full request/response detail
POST   /api/requests/:id/replay           # Replay with optional overrides
DELETE /api/requests                      # Clear captured requests (?app=)
GET    /api/requests/:id1/diff/:id2       # Side-by-side diff of two requests

# Mocks
GET    /api/mocks                         # List all mock rules
POST   /api/mocks                         # Create a mock rule
PUT    /api/mocks/:id                     # Update a mock rule
DELETE /api/mocks/:id                     # Delete a mock rule
PATCH  /api/mocks/:id/toggle              # Enable/disable a mock

# Intercepts
GET    /api/intercepts/rules              # List intercept rules
POST   /api/intercepts/rules              # Create intercept rule
DELETE /api/intercepts/rules/:id          # Delete intercept rule
GET    /api/intercepts/pending            # List pending intercepts
POST   /api/intercepts/:id/decide         # Forward/modify/drop a pending request
       { "action": "forward" | "forward_modified" | "drop",
         "modifications": { "headers": {}, "body": "..." } }

# Network Simulation
GET    /api/network/:app                  # Get current network profile
PUT    /api/network/:app                  # Set network profile
DELETE /api/network/:app                  # Clear network simulation

# Tunnel
POST   /api/apps/:name/share             # Start a public tunnel
DELETE /api/apps/:name/share             # Stop a public tunnel

# System
GET    /api/status                        # Daemon health + stats

# WebSocket
WS     /api/ws                            # Real-time event stream
       Events:
       - request:start      { id, app, method, url, timestamp }
       - request:complete   { id, app, status, duration_ms }
       - app:registered     { name, port, pid, url }
       - app:removed        { name }
       - app:crashed        { name, exit_code }
       - app:restarted      { name, pid }
       - log:line           { app, stream, line, timestamp }
       - intercept:pending  { id, app, method, url }
       - intercept:decided  { id, action }
       - mock:hit           { mock_id, request_id }
       - tunnel:started     { app, public_url }
       - tunnel:stopped     { app }
```

---

### 5.12 Dashboard (`apps/desktop/`)

#### Dual-mode Architecture

The React SPA is **the same codebase** in both modes:

1. **Tauri desktop app** (primary): Webview loads the SPA. System tray, native notifications, global shortcuts. The Tauri Rust backend manages the daemon lifecycle.
2. **Web fallback**: The daemon embeds the built SPA as static files (via `rust-embed`) and serves them at `_portzero.localhost:1337/`. Works when Tauri is not installed.

```
┌─────────────────────────────────────────────┐
│          Tauri v2 Desktop App               │
│  ┌───────────────────────────────────────┐  │
│  │   React SPA (Vite)                    │  │
│  │   Connects to daemon API via HTTP/WS  │  │
│  └───────────────────┬───────────────────┘  │
│                      │                      │
│  Tauri Rust backend  │                      │
│  ┌───────────────────┴───────────────────┐  │
│  │ • Start/stop daemon on app launch     │  │
│  │ • System tray: list apps, open in     │  │
│  │   browser, restart, stop              │  │
│  │ • Native notifications: app crashed,  │  │
│  │   5xx response, intercept pending     │  │
│  │ • Global shortcut: Cmd+Shift+Z opens  │  │
│  │   PortZero from anywhere              │  │
│  │ • Auto-update via Tauri updater       │  │
│  └───────────────────────────────────────┘  │
└─────────────────────────────────────────────┘
```

#### Frontend Tech Stack

- **Vite** -- build tool
- **React 19** -- UI
- **TanStack Router** -- type-safe file-based routing
- **TanStack Query** -- server state + cache invalidation
- **Tailwind CSS v4** -- styling
- **Native WebSocket** -- real-time events
- **Lucide React** -- icons
- **Monaco Editor** -- request body editor, mock body editor (optional, lazy-loaded)

#### Dashboard Pages

| Page | Route | Purpose |
|---|---|---|
| Overview | `/` | App cards with status, resource usage, recent traffic |
| Traffic | `/traffic` | Full request log with filtering, sorting, search |
| Request Detail | `/traffic/:id` | Headers, body (formatted), timing, replay button |
| Request Diff | `/traffic/:id1/diff/:id2` | Side-by-side comparison of two requests |
| App Detail | `/apps/:name` | Logs, traffic for one app, resource graph, schema |
| Mocks | `/mocks` | List/create/toggle mock rules |
| Intercepts | `/intercepts` | Manage breakpoints, decide on pending intercepts |

---

## 6. CLI Commands

```
portzero <command>                        # Name = basename(cwd), run command
portzero <name> <command>                 # Explicit name, run command

portzero start                            # Start daemon only (foreground)
portzero start -d                         # Start daemon (background, Pingora built-in)
portzero stop                             # Graceful shutdown
portzero list                             # List active apps + URLs
portzero logs <name>                      # Tail logs for an app
portzero dashboard                        # Open dashboard (launches Tauri app or browser)

portzero up                               # Start all apps from portzero.toml
portzero down                             # Stop all apps

portzero mock <app> <method> <path> [opts]   # Create a response mock
portzero mock list                           # List mocks
portzero mock disable <id>                   # Disable a mock

portzero intercept <app> [opts]              # Set an intercept breakpoint
portzero intercept list                      # List breakpoints
portzero intercept clear                     # Remove all breakpoints

portzero throttle <app> [opts]               # Set network simulation
portzero throttle clear <app>                # Clear simulation

portzero share <app>                         # Start public tunnel via LocalUp
portzero share <app> --subdomain <name>      # Custom subdomain
portzero share <app> --relay <host:port>     # Use specific relay server
portzero mcp                                 # Start MCP server (stdio)

portzero --help
portzero --version
```

**Argument disambiguation** (same as Portless):
1. If first arg is a built-in command → treat as command
2. If first arg resolves as an executable in `$PATH` or `./node_modules/.bin` → treat as command, name = `basename(cwd)`
3. Otherwise → first arg is app name, rest is command

---

## 7. Config File

Optional `portzero.toml` for multi-service setups:

```toml
[proxy]
port = 1337
https = true

[apps.web]
command = "pnpm dev"
cwd = "./apps/web"
auto_restart = true

[apps.web.env]
NODE_ENV = "development"

[apps.api]
command = "pnpm start"
cwd = "./apps/api"
subdomain = "api.myapp"

[apps.docs]
command = "pnpm dev"
cwd = "./apps/docs"

[tunnel]
relay = "relay.portzero.dev:4443"    # Default LocalUp relay server
# token = "your-jwt-token"          # Or set PORTZERO_TUNNEL_TOKEN env var
# transport = "quic"                # quic (default), websocket, or h2
```

`portzero up` reads this file and starts all defined apps. `portzero down` stops them.

---

## 8. Data Flow Diagrams

### 8.1 Request with Intercept + Mock Check

```
Browser: POST https://my-app.localhost:1337/api/webhook
                     │
                     ▼
            Pingora ProxyHttp
            upstream_peer() → resolve "my-app" → port 4001
                     │
                     ▼
            request_filter()
            ┌────────┴────────┐
            │                 │
        Mock match?     Intercept match?
          ┌─Yes─┐          ┌─Yes─┐
          │     │          │     │
     Return    No     Pause request
     mock resp  │     WS → dashboard
          │     │     Wait for decision
          ▼     │          │
       (done)   │     ┌────┴────┐
                │   Forward  Modify  Drop
                │     │       │       │
                └──┬──┘   Apply    Return
                   │      edits    444
                   ▼        │
         upstream_request_filter()
         Apply network simulation
         (latency, loss)
                   │
                   ▼
         Forward to 127.0.0.1:4001
                   │
                   ▼
         upstream_response_filter()
         Capture response headers
                   │
                   ▼
         upstream_response_body_filter()
         Capture response body chunks
                   │
                   ▼
         logging()
         Save to SQLite
         Feed schema inference
         Broadcast WS event
                   │
                   ▼
         Response to browser
```

### 8.2 MCP Agent Debugging Flow

```
AI Agent: "The signup form returns a 500"
                     │
                     ▼
         MCP: list_apps()
         → [{ name: "web", status: "running" },
            { name: "api", status: "running" }]
                     │
                     ▼
         MCP: get_recent_requests(app="api", status_range="5xx")
         → [{ id: "abc-123", method: "POST", path: "/api/users",
              status: 500, duration: 45 }]
                     │
                     ▼
         MCP: get_request_detail(id="abc-123")
         → { request: { headers: {...}, body: {...} },
             response: { status: 500,
                         body: { error: "duplicate key: email" } } }
                     │
                     ▼
         Agent reads code, finds missing unique constraint
         Agent fixes the code
                     │
                     ▼
         MCP: replay_request(id="abc-123")
         → { status: 201, body: { id: 42, name: "John" } }
                     │
                     ▼
         Agent: "Fixed. The POST /api/users now returns 201."
```

---

## 9. Tech Stack

| Component | Technology | Rationale |
|---|---|---|
| Proxy engine | **Pingora** (`pingora-proxy`) | Battle-tested, HTTP/1+2, WebSocket, graceful reload, connection pooling |
| Async runtime | **Tokio** | Pingora's runtime, industry standard |
| API server | **axum** | Tokio-native, fast, ergonomic, tower middleware |
| CLI argument parsing | **clap** | Derive macro, subcommands, great UX |
| SQLite | **rusqlite** | Zero-overhead C bindings, WAL mode, `r2d2` pool |
| TLS certs | **rcgen** + **rustls** | Pure Rust cert generation, no OpenSSL dependency |
| Serialization | **serde** + **serde_json** | Standard, zero-copy deserialization |
| Config parsing | **toml** | Human-friendly config format |
| UUID generation | **uuid** | Request ID generation |
| Concurrent maps | **dashmap** | Lock-free concurrent HashMap for hot paths |
| MCP protocol | **rmcp** or hand-rolled | stdio JSON-RPC per MCP spec |
| Tunnel | **localup-lib** (`localup-dev/localup`) | Rust-native, QUIC/WS/H2 transports, self-hostable relay, JWT auth |
| Desktop app | **Tauri v2** | Native webview (~8MB), system tray, notifications, auto-update |
| Dashboard framework | **React 19** + **Vite** | Fast dev + optimized production builds |
| Dashboard routing | **TanStack Router** | Type-safe, file-based |
| Dashboard state | **TanStack Query** | Server state caching + real-time invalidation |
| Dashboard styling | **Tailwind CSS v4** | Utility-first, small bundle |
| Monorepo | **Cargo workspace** + **pnpm** (for UI) | Rust workspace for crates, pnpm for dashboard JS |
| Testing | **cargo test** + **Vitest** | Rust tests for core, Vitest for dashboard |
| CI | **GitHub Actions** | Cross-platform builds (macOS, Linux, Windows via Tauri) |

---

## 10. Build & Distribution

### Binary Distribution

PortZero ships as a **single static binary** per platform. No Node.js, no npm, no runtime dependencies.

```
Build pipeline:

  1. Dashboard UI:    pnpm build  →  ui/dist/
  2. Tauri app:       cargo tauri build  →  .dmg / .AppImage / .msi
  3. CLI-only binary: cargo build --release -p portzero-cli
                      (embeds dashboard via rust-embed)

Distribution:
  - GitHub Releases: portzero-cli binary + Tauri installers
  - cargo install portzero-cli
  - Homebrew: brew install portzero
  - Tauri auto-update for desktop app
```

### Embedded Dashboard

The CLI binary embeds the built dashboard SPA using `rust-embed`:

```rust
#[derive(RustEmbed)]
#[folder = "../../dashboard-dist"]
struct DashboardAssets;

// In the axum API server:
async fn serve_dashboard(uri: Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };

    match DashboardAssets::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            ([(header::CONTENT_TYPE, mime.as_ref())], content.data).into_response()
        }
        None => {
            // SPA fallback: serve index.html for client-side routing
            let index = DashboardAssets::get("index.html").unwrap();
            ([(header::CONTENT_TYPE, "text/html")], index.data).into_response()
        }
    }
}
```

---

## 11. State Directory Layout

```
~/.portzero/
├── portzero.db                    # SQLite database (requests, mocks, settings)
├── portzero.db-wal                # SQLite WAL file
├── daemon.pid                     # Daemon process ID
├── daemon.port                    # Daemon listening port
├── daemon.log                     # Daemon stdout/stderr
├── certs/
│   ├── ca.key                     # Local CA private key
│   ├── ca.crt                     # Local CA certificate
│   ├── server.key                 # Server private key
│   └── server.crt                 # Server certificate
└── logs/
    ├── my-app.log                 # Per-app log files (optional)
    └── api.log
```

---

## 12. Security Considerations

- Proxy binds to `127.0.0.1` only -- not network-accessible
- `_portzero` is a reserved subdomain; apps cannot register it
- Request bodies capped at 1MB to prevent memory exhaustion
- SQLite database at `~/.portzero/` with `0600` permissions
- Auto-generated TLS CA scoped to `*.localhost` only
- Replay endpoint only sends to `127.0.0.1` -- not an SSRF vector
- Tunnel traffic is end-to-end encrypted (LocalUp QUIC/TLS) with JWT-authenticated relay connections
- MCP server runs on stdio only -- no network exposure
- WebSocket unauthenticated (localhost-only, acceptable for dev tooling)

---

## 13. Pingora Integration Details

### Server Lifecycle

PortZero uses Pingora's `Server` for daemon management:

```rust
use pingora::prelude::*;

fn main() {
    let mut server = Server::new(Some(Opt::parse_args())).unwrap();
    server.bootstrap();

    // 1. Proxy service (Pingora)
    let proxy = PortZeroProxy::new(/* ... */);
    let mut proxy_service = http_proxy_service(&server.configuration, proxy);
    proxy_service.add_tcp("127.0.0.1:1337");

    // 2. API service (axum, runs as a Pingora background service)
    let api_service = background_service("api", ApiTask::new(/* ... */));

    // 3. MCP service (optional, if --mcp flag)

    server.add_service(proxy_service);
    server.add_service(api_service);
    server.run_forever();
}
```

This gives us for free:
- **Daemonization**: `portzero start -d` forks to background
- **Graceful shutdown**: `portzero stop` sends SIGTERM, in-flight requests drain
- **Graceful upgrade**: SIGQUIT + restart with `-u` flag, zero-downtime binary upgrade
- **PID file management**: Built into Pingora's `Server`
- **Multi-threaded runtime**: Configurable thread count

### Reserved Subdomain Routing

The `_portzero` subdomain is handled by routing to the axum API server instead of an upstream app. This is done in `upstream_peer()`:

```rust
async fn upstream_peer(&self, session: &mut Session, ctx: &mut Self::CTX) -> Result<Box<HttpPeer>> {
    let subdomain = extract_subdomain(host);

    if subdomain == "_portzero" {
        // Route to internal axum API server running on a Unix socket or internal port
        return Ok(Box::new(HttpPeer::new(
            "127.0.0.1:13370",  // internal API port
            false,
            String::new(),
        )));
    }

    // ... normal app routing
}
```

---

## 14. Future Considerations

These are architecturally accounted for but not part of the initial build:

- **Plugin system** -- Pingora's `ProxyHttp` trait naturally supports middleware via the callback chain. A plugin API could expose a subset of callbacks.
- **HAR export** -- Export captured traffic as HAR files for browser DevTools or Postman import
- **Team sharing** -- Export/import mock rules + captured request collections as JSON
- **CI mode** -- `portzero ci` runs headless, captures traffic during test suite, outputs report
- **gRPC support** -- Pingora supports gRPC proxying; schema inference could support protobuf
- **Windows support** -- Pingora has preliminary Windows support; Tauri is cross-platform. Main blocker is process management (`/proc` vs Windows APIs).
