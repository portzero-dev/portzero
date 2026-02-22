import { isTauri, apiBaseUrl } from "../lib/env";
import type {
  AppInfo,
  LogLine,
  RequestSummary,
  RequestDetail,
  RequestFilters,
  RequestDiff,
  ReplayOptions,
  MockRule,
  CreateMockRule,
  UpdateMockRule,
  NetworkProfile,
  UpdateNetworkProfile,
  InferredSchema,
  TunnelInfo,
  DaemonStatus,
  DaemonRunInfo,
} from "../lib/types";

// ---------------------------------------------------------------------------
// Error wrapper
// ---------------------------------------------------------------------------

export class ApiError extends Error {
  constructor(
    public status: number,
    public body: string,
    public path: string,
  ) {
    super(`API error on ${path}: ${body}`);
    this.name = "ApiError";
  }
}

// ---------------------------------------------------------------------------
// Transport helpers
// ---------------------------------------------------------------------------

/** Lazy-load Tauri invoke only when running inside Tauri. */
async function tauriInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke(cmd, args);
}

/** HTTP fetch helper for the daemon's REST API. */
async function httpFetch<T>(
  path: string,
  options: RequestInit = {},
): Promise<T> {
  const url = `${apiBaseUrl()}${path}`;
  const resp = await fetch(url, {
    headers: { "Content-Type": "application/json", ...options.headers as Record<string, string> },
    ...options,
  });
  if (!resp.ok) {
    const body = await resp.text();
    throw new ApiError(resp.status, body, path);
  }
  // 204 No Content
  if (resp.status === 204) return undefined as T;
  return resp.json();
}

// ---------------------------------------------------------------------------
// Apps
// ---------------------------------------------------------------------------

export async function listApps(): Promise<AppInfo[]> {
  if (isTauri()) return tauriInvoke("list_apps");
  return httpFetch("/apps");
}

export async function getApp(name: string): Promise<AppInfo> {
  if (isTauri()) return tauriInvoke("get_app", { name });
  return httpFetch(`/apps/${encodeURIComponent(name)}`);
}

export async function restartApp(name: string): Promise<void> {
  if (isTauri()) return tauriInvoke("restart_app", { name });
  return httpFetch(`/apps/${encodeURIComponent(name)}/restart`, { method: "POST" });
}

export async function stopApp(name: string): Promise<void> {
  if (isTauri()) return tauriInvoke("stop_app", { name });
  return httpFetch(`/apps/${encodeURIComponent(name)}/stop`, { method: "POST" });
}

export async function getAppLogs(
  name: string,
  lines = 100,
): Promise<LogLine[]> {
  if (isTauri()) return tauriInvoke("get_app_logs", { name, lines });
  return httpFetch(`/apps/${encodeURIComponent(name)}/logs?lines=${lines}`);
}

export async function getAppSchema(name: string): Promise<InferredSchema> {
  if (isTauri()) return tauriInvoke("get_app_schema", { name });
  return httpFetch(`/apps/${encodeURIComponent(name)}/schema`);
}

// ---------------------------------------------------------------------------
// Traffic / Requests
// ---------------------------------------------------------------------------

export async function listRequests(
  filters?: RequestFilters,
): Promise<RequestSummary[]> {
  if (isTauri()) return tauriInvoke("list_requests", { filters: filters ?? null });
  const params = new URLSearchParams();
  if (filters) {
    for (const [k, v] of Object.entries(filters)) {
      if (v != null) params.set(k, String(v));
    }
  }
  const qs = params.toString();
  return httpFetch(`/requests${qs ? `?${qs}` : ""}`);
}

export async function getRequest(id: string): Promise<RequestDetail> {
  if (isTauri()) return tauriInvoke("get_request", { id });
  return httpFetch(`/requests/${encodeURIComponent(id)}`);
}

export async function replayRequest(
  id: string,
  options?: ReplayOptions,
): Promise<RequestDetail> {
  if (isTauri()) return tauriInvoke("replay_request", { id, options: options ?? null });
  return httpFetch(`/requests/${encodeURIComponent(id)}/replay`, {
    method: "POST",
    body: options ? JSON.stringify(options) : undefined,
  });
}

export async function clearRequests(app?: string): Promise<void> {
  if (isTauri()) return tauriInvoke("clear_requests", { app: app ?? null });
  const qs = app ? `?app=${encodeURIComponent(app)}` : "";
  return httpFetch(`/requests${qs}`, { method: "DELETE" });
}

export async function diffRequests(
  id1: string,
  id2: string,
): Promise<RequestDiff> {
  if (isTauri()) return tauriInvoke("diff_requests", { id1, id2 });
  return httpFetch(`/requests/${encodeURIComponent(id1)}/diff/${encodeURIComponent(id2)}`);
}

// ---------------------------------------------------------------------------
// Mocks
// ---------------------------------------------------------------------------

export async function listMocks(): Promise<MockRule[]> {
  if (isTauri()) return tauriInvoke("list_mocks");
  return httpFetch("/mocks");
}

export async function createMock(rule: CreateMockRule): Promise<MockRule> {
  if (isTauri()) return tauriInvoke("create_mock", { rule });
  return httpFetch("/mocks", { method: "POST", body: JSON.stringify(rule) });
}

export async function updateMock(
  id: string,
  updates: UpdateMockRule,
): Promise<MockRule> {
  if (isTauri()) return tauriInvoke("update_mock", { id, updates });
  return httpFetch(`/mocks/${encodeURIComponent(id)}`, {
    method: "PUT",
    body: JSON.stringify(updates),
  });
}

export async function deleteMock(id: string): Promise<void> {
  if (isTauri()) return tauriInvoke("delete_mock", { id });
  return httpFetch(`/mocks/${encodeURIComponent(id)}`, { method: "DELETE" });
}

export async function toggleMock(id: string): Promise<MockRule> {
  if (isTauri()) return tauriInvoke("toggle_mock", { id });
  return httpFetch(`/mocks/${encodeURIComponent(id)}/toggle`, { method: "PATCH" });
}

// ---------------------------------------------------------------------------
// Network Simulation
// ---------------------------------------------------------------------------

export async function getNetworkProfile(
  app: string,
): Promise<NetworkProfile> {
  if (isTauri()) return tauriInvoke("get_network_profile", { app });
  return httpFetch(`/network/${encodeURIComponent(app)}`);
}

export async function updateNetworkProfile(
  app: string,
  profile: UpdateNetworkProfile,
): Promise<NetworkProfile> {
  if (isTauri()) return tauriInvoke("update_network_profile", { app, profile });
  return httpFetch(`/network/${encodeURIComponent(app)}`, {
    method: "PUT",
    body: JSON.stringify(profile),
  });
}

export async function clearNetworkProfile(app: string): Promise<void> {
  if (isTauri()) return tauriInvoke("clear_network_profile", { app });
  return httpFetch(`/network/${encodeURIComponent(app)}`, { method: "DELETE" });
}

// ---------------------------------------------------------------------------
// Tunnel
// ---------------------------------------------------------------------------

export async function startTunnel(
  app: string,
  subdomain?: string,
): Promise<TunnelInfo> {
  if (isTauri()) return tauriInvoke("start_tunnel", { app, subdomain: subdomain ?? null });
  return httpFetch(`/apps/${encodeURIComponent(app)}/share`, {
    method: "POST",
    body: subdomain ? JSON.stringify({ subdomain }) : undefined,
  });
}

export async function stopTunnel(app: string): Promise<void> {
  if (isTauri()) return tauriInvoke("stop_tunnel", { app });
  return httpFetch(`/apps/${encodeURIComponent(app)}/share`, { method: "DELETE" });
}

// ---------------------------------------------------------------------------
// System
// ---------------------------------------------------------------------------

export async function getDaemonStatus(): Promise<DaemonStatus> {
  if (isTauri()) return tauriInvoke("get_status");
  return httpFetch("/status");
}

export async function getDaemonInfo(): Promise<DaemonRunInfo> {
  if (isTauri()) return tauriInvoke("get_daemon_info");
  // In web mode, if we can reach the API, the daemon is running and responsive.
  return { running: true, pid: null, responsive: true };
}

export async function startDaemon(): Promise<void> {
  if (isTauri()) return tauriInvoke("start_daemon");
  // Not available in web mode — daemon is already running (we're served by it)
  throw new Error("Cannot start daemon from the web dashboard — it's already running.");
}

export async function stopDaemon(): Promise<void> {
  if (isTauri()) return tauriInvoke("stop_daemon");
  throw new Error("Cannot stop daemon from the web dashboard — you would lose access to this page.");
}

export async function restartDaemon(): Promise<void> {
  if (isTauri()) return tauriInvoke("restart_daemon");
  throw new Error("Cannot restart daemon from the web dashboard.");
}

// ---------------------------------------------------------------------------
// Certificates
// ---------------------------------------------------------------------------

export interface CertStatus {
  certs_exist: boolean;
  ca_trusted: boolean;
  ca_cert_path: string;
  trust_command: string;
}

export interface TrustResponse {
  status: string;
  message: string;
}

export async function getCertStatus(): Promise<CertStatus> {
  if (isTauri()) return tauriInvoke("get_cert_status");
  return httpFetch("/certs/status");
}

export async function trustCA(): Promise<TrustResponse> {
  if (isTauri()) return tauriInvoke("trust_ca");
  return httpFetch("/certs/trust", { method: "POST" });
}

export async function untrustCA(): Promise<TrustResponse> {
  if (isTauri()) return tauriInvoke("untrust_ca");
  return httpFetch("/certs/untrust", { method: "POST" });
}

// ---------------------------------------------------------------------------
// CLI Installation (Tauri-only — no-op stubs in web mode)
// ---------------------------------------------------------------------------

export interface CliStatus {
  installed: boolean;
  current_path: string | null;
  binary_path: string | null;
  binary_exists: boolean;
  install_dir: string;
}

export interface CliInstallResult {
  success: boolean;
  message: string;
  installed_path: string | null;
}

export async function getCliStatus(): Promise<CliStatus> {
  if (isTauri()) return tauriInvoke("get_cli_status");
  // In web mode, CLI management isn't available — return a sensible default
  return {
    installed: true,
    current_path: null,
    binary_path: null,
    binary_exists: true,
    install_dir: "",
  };
}

export async function installCli(
  installDir?: string,
): Promise<CliInstallResult> {
  if (isTauri()) return tauriInvoke("install_cli", { installDir: installDir ?? null });
  throw new Error("CLI installation is only available from the desktop app.");
}

export async function uninstallCli(): Promise<CliInstallResult> {
  if (isTauri()) return tauriInvoke("uninstall_cli");
  throw new Error("CLI uninstallation is only available from the desktop app.");
}
