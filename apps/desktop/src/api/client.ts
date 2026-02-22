import { invoke } from "@tauri-apps/api/core";
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

// Error wrapper to keep the same interface for callers
export class ApiError extends Error {
  constructor(
    public status: number,
    public body: string,
    public path: string,
  ) {
    super(`IPC error on ${path}: ${body}`);
    this.name = "ApiError";
  }
}

// ── Apps ────────────────────────────────────────────────────────────────────────

export async function listApps(): Promise<AppInfo[]> {
  return invoke("list_apps");
}

export async function getApp(name: string): Promise<AppInfo> {
  return invoke("get_app", { name });
}

export async function restartApp(name: string): Promise<void> {
  return invoke("restart_app", { name });
}

export async function stopApp(name: string): Promise<void> {
  return invoke("stop_app", { name });
}

export async function getAppLogs(
  name: string,
  lines = 100,
): Promise<LogLine[]> {
  return invoke("get_app_logs", { name, lines });
}

export async function getAppSchema(name: string): Promise<InferredSchema> {
  return invoke("get_app_schema", { name });
}

// ── Traffic / Requests ─────────────────────────────────────────────────────────

export async function listRequests(
  filters?: RequestFilters,
): Promise<RequestSummary[]> {
  return invoke("list_requests", { filters: filters ?? null });
}

export async function getRequest(id: string): Promise<RequestDetail> {
  return invoke("get_request", { id });
}

export async function replayRequest(
  id: string,
  options?: ReplayOptions,
): Promise<RequestDetail> {
  return invoke("replay_request", { id, options: options ?? null });
}

export async function clearRequests(app?: string): Promise<void> {
  return invoke("clear_requests", { app: app ?? null });
}

export async function diffRequests(
  id1: string,
  id2: string,
): Promise<RequestDiff> {
  return invoke("diff_requests", { id1, id2 });
}

// ── Mocks ──────────────────────────────────────────────────────────────────────

export async function listMocks(): Promise<MockRule[]> {
  return invoke("list_mocks");
}

export async function createMock(rule: CreateMockRule): Promise<MockRule> {
  return invoke("create_mock", { rule });
}

export async function updateMock(
  id: string,
  updates: UpdateMockRule,
): Promise<MockRule> {
  return invoke("update_mock", { id, updates });
}

export async function deleteMock(id: string): Promise<void> {
  return invoke("delete_mock", { id });
}

export async function toggleMock(id: string): Promise<MockRule> {
  return invoke("toggle_mock", { id });
}

// ── Network Simulation ─────────────────────────────────────────────────────────

export async function getNetworkProfile(
  app: string,
): Promise<NetworkProfile> {
  return invoke("get_network_profile", { app });
}

export async function updateNetworkProfile(
  app: string,
  profile: UpdateNetworkProfile,
): Promise<NetworkProfile> {
  return invoke("update_network_profile", { app, profile });
}

export async function clearNetworkProfile(app: string): Promise<void> {
  return invoke("clear_network_profile", { app });
}

// ── Tunnel ──────────────────────────────────────────────────────────────────────

export async function startTunnel(
  app: string,
  subdomain?: string,
): Promise<TunnelInfo> {
  return invoke("start_tunnel", { app, subdomain: subdomain ?? null });
}

export async function stopTunnel(app: string): Promise<void> {
  return invoke("stop_tunnel", { app });
}

// ── System ──────────────────────────────────────────────────────────────────────

export async function getDaemonStatus(): Promise<DaemonStatus> {
  return invoke("get_status");
}

export async function getDaemonInfo(): Promise<DaemonRunInfo> {
  return invoke("get_daemon_info");
}

export async function startDaemon(): Promise<void> {
  return invoke("start_daemon");
}

export async function stopDaemon(): Promise<void> {
  return invoke("stop_daemon");
}

export async function restartDaemon(): Promise<void> {
  return invoke("restart_daemon");
}

// ── Certificates ───────────────────────────────────────────────────────────────

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
  return invoke("get_cert_status");
}

export async function trustCA(): Promise<TrustResponse> {
  return invoke("trust_ca");
}

export async function untrustCA(): Promise<TrustResponse> {
  return invoke("untrust_ca");
}

// ── CLI Installation ───────────────────────────────────────────────────────────

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
  return invoke("get_cli_status");
}

export async function installCli(
  installDir?: string,
): Promise<CliInstallResult> {
  return invoke("install_cli", { installDir: installDir ?? null });
}

export async function uninstallCli(): Promise<CliInstallResult> {
  return invoke("uninstall_cli");
}
