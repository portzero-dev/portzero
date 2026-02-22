/**
 * Runtime environment detection.
 *
 * When the dashboard runs inside Tauri, `window.__TAURI_INTERNALS__` is set.
 * When served by the daemon's embedded web server, it's a plain browser.
 */
export function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

/**
 * Base URL for the HTTP API when running in web mode.
 *
 * The dashboard is served at `_portzero.localhost:<port>`, and the API
 * is at `/api/*` on the same origin, so we just use a relative path.
 */
export function apiBaseUrl(): string {
  return "/api";
}

/**
 * WebSocket URL for real-time events when running in web mode.
 */
export function wsUrl(): string {
  const proto = window.location.protocol === "https:" ? "wss:" : "ws:";
  return `${proto}//${window.location.host}/api/ws`;
}
