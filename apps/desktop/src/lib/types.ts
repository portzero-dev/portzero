// ── App Types ──────────────────────────────────────────────────────────────────

export type AppStatus =
  | { type: "running" }
  | { type: "crashed"; exit_code: number; at: string }
  | { type: "stopped" };

export interface AppInfo {
  name: string;
  port: number;
  pid: number | null;
  url: string;
  command: string[];
  cwd: string;
  status: AppStatus;
  started_at: string | null;
  restarts: number;
  auto_restart: boolean;
  cpu_usage: number | null;
  memory_bytes: number | null;
  tunnel_url: string | null;
}

export interface LogLine {
  timestamp: string;
  stream: "stdout" | "stderr";
  content: string;
}

// ── Request / Traffic Types ────────────────────────────────────────────────────

export interface RequestSummary {
  id: string;
  app_name: string;
  method: string;
  path: string;
  status_code: number;
  duration_ms: number;
  timestamp: string;
  mocked: boolean;
}

export interface RequestDetail {
  id: string;
  app_name: string;
  timestamp: string;
  duration_ms: number;

  // Request
  method: string;
  url: string;
  path: string;
  query_string: string;
  request_headers: Record<string, string>;
  request_body: string | null;
  request_content_type: string | null;

  // Response
  status_code: number;
  status_message: string;
  response_headers: Record<string, string>;
  response_body: string | null;
  response_content_type: string | null;

  // Metadata
  mocked: boolean;
  parent_id: string | null;
}

export interface RequestFilters {
  app?: string;
  method?: string;
  status?: number;
  status_range?: string;
  path?: string;
  search?: string;
  from?: number;
  to?: number;
  limit?: number;
  offset?: number;
}

export interface RequestDiff {
  left: RequestDetail;
  right: RequestDetail;
}

export interface ReplayOptions {
  headers?: Record<string, string>;
  body?: string;
  method?: string;
  path?: string;
}

// ── Mock Types ─────────────────────────────────────────────────────────────────

export interface MockRule {
  id: string;
  app_name: string;
  method: string | null;
  path_pattern: string;
  status_code: number;
  response_headers: Record<string, string>;
  response_body: string;
  enabled: boolean;
  hit_count: number;
}

export interface CreateMockRule {
  app_name: string;
  method?: string;
  path_pattern: string;
  status_code: number;
  response_headers?: Record<string, string>;
  response_body: string;
}

export interface UpdateMockRule {
  method?: string;
  path_pattern?: string;
  status_code?: number;
  response_headers?: Record<string, string>;
  response_body?: string;
}

// ── Network Simulation Types ───────────────────────────────────────────────────

export interface NetworkProfile {
  app_name: string;
  latency_ms: number | null;
  latency_jitter_ms: number | null;
  packet_loss_rate: number;
  bandwidth_limit_bytes: number | null;
  path_filter: string | null;
}

export interface UpdateNetworkProfile {
  latency_ms?: number;
  latency_jitter_ms?: number;
  packet_loss_rate?: number;
  bandwidth_limit_bytes?: number;
  path_filter?: string;
}

// ── Schema Types ───────────────────────────────────────────────────────────────

export interface InferredSchema {
  app_name: string;
  endpoints: InferredEndpoint[];
  last_updated: string;
}

export interface InferredEndpoint {
  method: string;
  path_template: string;
  query_params: ParamInfo[];
  request_body_schema: JsonSchema | null;
  response_schemas: Record<number, JsonSchema>;
  sample_count: number;
}

export interface ParamInfo {
  name: string;
  type: string;
  required: boolean;
  example_values: string[];
}

export interface JsonSchema {
  type: string;
  properties?: Record<string, JsonSchema>;
  items?: JsonSchema;
  required?: string[];
  enum?: string[];
}

// ── Tunnel Types ───────────────────────────────────────────────────────────────

export interface TunnelInfo {
  app_name: string;
  public_url: string;
  relay: string;
  transport: string;
  started_at: string;
}

// ── System Types ───────────────────────────────────────────────────────────────

export interface DaemonStatus {
  version: string;
  uptime_seconds: number;
  app_count: number;
  total_requests: number;
  proxy_port: number;
}

export interface DaemonRunInfo {
  /** Whether the daemon process is alive. */
  running: boolean;
  /** The daemon's PID (if known from the PID file). */
  pid: number | null;
  /** Whether the control socket is responsive (daemon is healthy). */
  responsive: boolean;
}

// ── WebSocket Event Types ──────────────────────────────────────────────────────

export type WsEvent =
  | { type: "request:start"; id: string; app: string; method: string; url: string; timestamp: string }
  | { type: "request:complete"; id: string; app: string; status: number; duration_ms: number }
  | { type: "app:registered"; name: string; port: number; pid: number; url: string }
  | { type: "app:removed"; name: string }
  | { type: "app:crashed"; name: string; exit_code: number }
  | { type: "app:restarted"; name: string; pid: number }
  | { type: "log:line"; app: string; stream: "stdout" | "stderr"; line: string; timestamp: string }
  | { type: "mock:hit"; mock_id: string; request_id: string }
  | { type: "tunnel:started"; app: string; public_url: string }
  | { type: "tunnel:stopped"; app: string };
